use std::{path::Path, sync::Arc};

use arrow_array::{
    ArrayRef, Float32Array, RecordBatch, RecordBatchIterator, StringArray,
    builder::{FixedSizeListBuilder, Float32Builder},
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use futures::TryStreamExt;
use lancedb::{
    Connection, DistanceType, Result, Table, connect,
    query::{ExecutableQuery, QueryBase},
};
use uuid::Uuid;

use crate::common::data::{Chunk, ChunkInfo, IndexType, QueryResult};

/// Placeholder for a LanceDB-backed vector store; wire [`lancedb`] APIs here.
pub struct LancedbChunkStore {
    conn: Connection,
    schema: SchemaRef,
    dim: i32,
}

impl LancedbChunkStore {
    pub async fn open(db_path: &Path, dim: i32) -> Result<Self> {
        let uri = db_path.to_string_lossy();
        let conn = connect(&uri).execute().await?;

        let schema = Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("layer", DataType::Utf8, false),
            Field::new("file_path", DataType::Utf8, false),
            Field::new("lang", DataType::Utf8, false),
            Field::new("chunk_info", DataType::Utf8, false),
            Field::new(
                "embedding",
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), dim),
                false,
            ),
        ]);
        Ok(Self {
            conn,
            schema: SchemaRef::new(schema),
            dim,
        })
    }

    pub async fn get_or_create_table(&self, identifier: &str, layer: IndexType) -> Result<Table> {
        let table_name = format!("{}_{}", layer.to_string(), identifier);
        let table = self.conn.table_names().execute().await?;

        if table.contains(&table_name) {
            let table = self.conn.open_table(&table_name).execute().await?;
            Ok(table)
        } else {
            let table = self
                .conn
                .create_empty_table(&table_name, self.schema.clone())
                .execute()
                .await?;
            Ok(table)
        }
    }

    pub async fn append_chunks(
        &self,
        identifier: &str,
        layer: IndexType,
        chunks: Vec<Chunk>,
    ) -> Result<()> {
        let table = self.get_or_create_table(identifier, layer).await?;

        let mut ids = Vec::new();
        let mut layers = Vec::new();
        let mut file_paths = Vec::new();
        let mut langs = Vec::new();
        let mut chunk_infos = Vec::new();

        let vector_item_builder = Float32Builder::new();
        let mut vector_builder = FixedSizeListBuilder::new(vector_item_builder, self.dim);

        for chunk in chunks {
            let id = Uuid::new_v4().to_string();

            ids.push(id);
            layers.push(chunk.info.layer.to_string());
            file_paths.push(chunk.info.file_path.clone());
            langs.push(chunk.info.lang.clone());
            chunk_infos.push(serde_json::to_string(&chunk.info).unwrap_or_default());

            vector_builder.values().append_slice(&chunk.embedding);
            vector_builder.append(true);
        }

        let id_array: ArrayRef = Arc::new(StringArray::from(ids));
        let layer_array: ArrayRef = Arc::new(StringArray::from(layers));
        let file_array: ArrayRef = Arc::new(StringArray::from(file_paths));
        let lang_array: ArrayRef = Arc::new(StringArray::from(langs));
        let chunk_info_array: ArrayRef = Arc::new(StringArray::from(chunk_infos));
        let embedding_array: ArrayRef = Arc::new(vector_builder.finish());

        let batch = RecordBatch::try_new(
            self.schema.clone(),
            vec![
                id_array,
                layer_array,
                file_array,
                lang_array,
                chunk_info_array,
                embedding_array,
            ],
        )?;

        let batch_iter = RecordBatchIterator::new(vec![Ok(batch)], self.schema.clone());

        table.add(Box::new(batch_iter)).execute().await?;

        Ok(())
    }

    pub async fn search(
        &self,
        identifier: &str,
        query_vector: Vec<f32>,
        limit: usize,
        threshold: f32,
        layer: IndexType,
        paths: Vec<String>,
    ) -> Result<Vec<QueryResult>> {
        let table = self.get_or_create_table(identifier, layer).await?;

        let mut search = table
            .query()
            .nearest_to(query_vector)?
            .limit(limit)
            .distance_type(DistanceType::Cosine);

        if let Some(filter) = construct_filter(&paths) {
            search = search.only_if(filter);
        }

        let results = search.execute().await?;
        let batches: Vec<RecordBatch> = results.try_collect().await?;

        let mut output = Vec::new();
        for batch in batches {
            let info_array = batch
                .column_by_name("chunk_info")
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();

            let distance_array = batch
                .column_by_name("_distance")
                .unwrap()
                .as_any()
                .downcast_ref::<Float32Array>()
                .unwrap();

            for i in 0..batch.num_rows() {
                let distance = distance_array.value(i);
                let score = 1.0 - distance;

                if score < threshold {
                    continue;
                }

                let info = info_array.value(i);
                let info = serde_json::from_str::<ChunkInfo>(&info).unwrap_or_default();
                output.push(QueryResult { score, info });
            }
        }

        Ok(output)
    }

    pub async fn delete_chunks_by_path(
        &self,
        identifier: &str,
        file_path: &str,
        layer: IndexType,
    ) -> Result<()> {
        self.delete_chunks_by_paths(identifier, vec![file_path.to_string()], layer)
            .await
    }

    pub async fn delete_chunks_by_paths(
        &self,
        identifier: &str,
        paths: Vec<String>,
        layer: IndexType,
    ) -> Result<()> {
        if paths.is_empty() {
            return Ok(());
        }

        let table = self.get_or_create_table(identifier, layer).await?;
        if let Some(filter) = construct_filter(&paths) {
            table.delete(&filter).await?;
        }

        Ok(())
    }

    pub async fn delete_table(&self, identifier: &str, layer: IndexType) -> Result<()> {
        let table_name = format!("{}_{}", layer.to_string(), identifier);
        let tables = self.conn.table_names().execute().await?;
        if tables.contains(&table_name) {
            self.conn.drop_table(&table_name).await?;
        }
        Ok(())
    }
}

fn escape(value: &str) -> String {
    value.replace('\'', "''")
}

fn construct_filter(paths: &[String]) -> Option<String> {
    if !paths.is_empty() {
        let path_list = paths
            .iter()
            .map(|path| format!("'{}'", escape(path)))
            .collect::<Vec<String>>()
            .join(", ");
        Some(format!("file_path IN ({})", path_list))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::test::utils::temp_dir;
    use anyhow;
    use std::str::FromStr;
    use tokio;

    use super::*;

    fn make_dummy_chunk(layer: &str, file_path: &str, embedding: Vec<f32>) -> Chunk {
        Chunk {
            embedding_content: "dummy".to_string(),
            info: ChunkInfo {
                file_path: file_path.to_string(),
                layer: IndexType::from_str(layer).unwrap(),
                lang: "rust".to_string(),
                content: Some("dummy content".to_string()),
                range: None,
            },
            embedding,
            ..Default::default()
        }
    }

    async fn setup_table(identifier: &str) -> Result<LancedbChunkStore> {
        let dir = temp_dir().join(format!("vectordb_{}", uuid::Uuid::new_v4()));
        let db = LancedbChunkStore::open(&dir, 2).await?;

        db.delete_table(identifier, IndexType::File).await?;
        db.delete_table(identifier, IndexType::Symbol).await?;

        db.get_or_create_table(identifier, IndexType::File).await?;
        db.get_or_create_table(identifier, IndexType::Symbol)
            .await?;

        let file_chunks = vec![
            make_dummy_chunk("file", "src/main.rs", vec![0.1, 0.2]),
            make_dummy_chunk("file", "src/lib.rs", vec![0.4, 0.5]),
        ];
        db.append_chunks(identifier, IndexType::File, file_chunks)
            .await?;

        let symbol_chunks = vec![
            make_dummy_chunk("symbol", "src/main.rs", vec![0.9, 0.9]),
            make_dummy_chunk("symbol", "src/main.rs", vec![0.8, 0.8]),
            make_dummy_chunk("symbol", "src/lib.rs", vec![0.3, 0.4]),
            make_dummy_chunk("symbol", "src/lib.rs", vec![0.3, 0.3]),
            make_dummy_chunk("symbol", "src/lib.rs", vec![0.4, 0.3]),
        ];
        db.append_chunks(identifier, IndexType::Symbol, symbol_chunks)
            .await?;

        Ok(db)
    }

    #[tokio::test]
    async fn test_search_layer_filer() -> anyhow::Result<()> {
        let identifier = "chunks";
        let db = setup_table(identifier).await?;

        let results = db
            .search(
                identifier,
                vec![0.3, 0.4],
                10,
                0.0,
                IndexType::Symbol,
                vec![],
            )
            .await?;

        assert_eq!(results.len(), 5);
        assert_eq!(results[0].info.file_path, "src/lib.rs");
        assert_eq!(results[0].info.layer, IndexType::Symbol);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_chunks() -> anyhow::Result<()> {
        let identifier = "chunks";
        let db = setup_table(identifier).await?;

        let layers = vec![IndexType::Symbol, IndexType::File];
        for layer in layers {
            db.delete_chunks_by_path(identifier, "src/lib.rs", layer)
                .await?;
        }

        let results = db
            .search(
                identifier,
                vec![0.3, 0.4],
                10,
                0.0,
                IndexType::Symbol,
                vec![],
            )
            .await?;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].info.file_path, "src/main.rs");
        assert_eq!(results[0].info.layer, IndexType::Symbol);
        Ok(())
    }
}
