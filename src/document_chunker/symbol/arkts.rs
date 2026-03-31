use std::path::Path;
use std::sync::Mutex;

use async_trait::async_trait;
use tree_sitter::Parser;

use crate::common::data::Chunk;
use crate::document_chunker::chunker::Chunker;

pub struct ArkTsChunker {
    parser: Mutex<Parser>,
}

impl ArkTsChunker {
    pub fn new() -> Self {
        Self {
            parser: Mutex::new(Parser::new()),
        }
    }
}

#[async_trait]
impl Chunker for ArkTsChunker {
    async fn split(&self, _path: &Path, _relative_path: &str) -> anyhow::Result<Vec<Chunk>> {
        Ok(vec![])
    }
}
