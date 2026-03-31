use async_trait::async_trait;
use std::path::Path;

use crate::common::data::{Chunk, ChunkInfo, IndexType};
use crate::document_chunker::chunker::Chunker;
use crate::language::language::Language;

pub struct FileChunker {}

impl FileChunker {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Chunker for FileChunker {
    async fn split(&self, path: &Path, relative_path: &str) -> anyhow::Result<Vec<Chunk>> {
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let language: Language = path.into();
        Ok(vec![Chunk {
            embedding_content: file_name,
            info: ChunkInfo {
                layer: IndexType::File,
                lang: language.id(),
                file_path: relative_path.to_string(),
                ..Default::default()
            },
            ..Default::default()
        }])
    }
}
