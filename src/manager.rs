use std::{path::Path, sync::Arc};

use tokio::runtime::Handle;

use crate::{
    common::data::{IndexType, Project, QueryResult},
    document_chunker::chunker::ChunkerRegistry,
    embedding::{
        onnx::OnnxEmbedder,
        utils::{Embedder, EmbeddingOptions, OnnxRuntimeConfig},
    },
    index::{
        manager::IndexManager,
        utils::{CancelToken, ProgressReporter},
    },
    metrics::profiler::IndexProfiler,
    storage::manager::{StorageManager, StorageOptions},
};

pub struct SemanticSearchOptions {
    pub storage_options: StorageOptions,
    pub onnx_runtime_path: String,
    pub embedding_options: EmbeddingOptions,
}

pub struct SemanticSearchManager {
    storage_manager: Arc<StorageManager>,
    embedder: Arc<OnnxEmbedder>,
    index_manager: Arc<IndexManager>,
    project: Project,
    runtime_handle: Handle,
}

impl SemanticSearchManager {
    pub async fn new(
        storage_options: StorageOptions,
        onnx_runtime_config: OnnxRuntimeConfig,
        embedding_options: EmbeddingOptions,
        project_path: &Path,
        runtime_handle: Handle,
    ) -> anyhow::Result<Self> {
        let storage_manager =
            Arc::new(StorageManager::new(storage_options, embedding_options.dim as i32).await?);

        OnnxEmbedder::load_runtime(&onnx_runtime_config.runtime_path)?;
        let embedder = Arc::new(OnnxEmbedder::new(
            onnx_runtime_config,
            embedding_options.clone(),
        )?);

        let mut chunker_registry = ChunkerRegistry::new();
        chunker_registry.register_chunkers();

        let index_manager = Arc::new(IndexManager::new(
            storage_manager.clone(),
            embedder.clone(),
            chunker_registry,
            runtime_handle.clone(),
            embedding_options.num_threads,
            embedding_options.batch_size,
        ));

        let project = storage_manager
            .get_or_create_project(project_path, &embedding_options.model_type.to_string())?;
        Ok(Self {
            storage_manager,
            embedder,
            index_manager,
            project,
            runtime_handle,
        })
    }

    pub fn storage_options(&self) -> StorageOptions {
        self.storage_manager.storage_options()
    }

    pub fn index(
        &self,
        progress_reporter: Arc<dyn ProgressReporter>,
        cancel_token: Arc<dyn CancelToken>,
    ) -> anyhow::Result<Arc<IndexProfiler>> {
        let profiler = Arc::new(IndexProfiler::new(self.storage_manager.storage_options()));

        for layer in vec![IndexType::File, IndexType::Symbol] {
            let profiler_clone = profiler.clone();
            let index_manager_clone = self.index_manager.clone();
            let progress_reporter_clone = progress_reporter.clone();
            let cancel_token_clone = cancel_token.clone();
            let project_clone = self.project.clone();

            self.runtime_handle.spawn(async move {
                let _ = index_manager_clone
                    .index_project(
                        &project_clone,
                        layer,
                        profiler_clone,
                        progress_reporter_clone,
                        cancel_token_clone,
                    )
                    .await;
            });
        }

        Ok(profiler)
    }

    pub async fn index_layer(
        &self,
        layer: IndexType,
        profiler: Arc<IndexProfiler>,
        progress_reporter: Arc<dyn ProgressReporter>,
        cancel_token: Arc<dyn CancelToken>,
    ) -> anyhow::Result<()> {
        self.index_manager
            .index_project(
                &self.project,
                layer,
                profiler,
                progress_reporter,
                cancel_token,
            )
            .await
    }

    pub async fn delete(&self) -> anyhow::Result<()> {
        for layer in vec![IndexType::File, IndexType::Symbol] {
            self.index_manager
                .delete_project(&self.project, layer)
                .await?;
        }
        Ok(())
    }

    pub async fn delete_layer(&self, layer: IndexType) -> anyhow::Result<()> {
        self.index_manager
            .delete_project(&self.project, layer)
            .await
    }

    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        threshold: f32,
        layer: IndexType,
        paths: Vec<String>,
    ) -> anyhow::Result<Vec<QueryResult>> {
        let query_vector = self.embedder.embed(query)?;
        self.storage_manager
            .search(
                &self.project.hash,
                query_vector,
                limit,
                threshold,
                layer,
                paths,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::test::utils::{cancel_token, construct_manager, index_reporter, init_log};

    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_search() {
        init_log();

        let manager = construct_manager().await.unwrap();
        let index_reporter = index_reporter();
        let cancel_token = cancel_token();

        // manager.delete().await.unwrap();
        let profiler = manager.index(index_reporter, cancel_token).unwrap();

        loop {
            if profiler.is_finished() {
                let metrics = profiler.stop_profiler();
                log::info!("indexing metrics: {}", metrics);
                break;
            }
        }

        let search_results = manager
            .search("audioplayer", 10, 0.5, IndexType::File, vec![])
            .await
            .unwrap();
        log::info!("search results: {:?}", search_results);
    }
}
