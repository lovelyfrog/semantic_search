use std::path::Path;

use async_trait::async_trait;
use lsp_types::Position;
use parking_lot::Mutex;
use tree_sitter::Parser;

use crate::common::FileService;
use crate::common::data::Chunk;
use crate::document_chunker::chunker::Chunker;
use crate::document_chunker::symbol::{SymbolKind, SymbolPipeline};
use crate::language::language::Language;

pub struct TsChunker {
    parser_ts: Mutex<Parser>,
    parser_tsx: Mutex<Parser>,
    file_service: FileService,
    pipeline: SymbolPipeline,
}

impl TsChunker {
    pub fn new() -> anyhow::Result<Self> {
        let mut parser_ts = Parser::new();
        parser_ts
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .map_err(|e| anyhow::anyhow!("tree-sitter TypeScript grammar: {e}"))?;
        let mut parser_tsx = Parser::new();
        parser_tsx
            .set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
            .map_err(|e| anyhow::anyhow!("tree-sitter TSX grammar: {e}"))?;
        Ok(Self {
            parser_ts: Mutex::new(parser_ts),
            parser_tsx: Mutex::new(parser_tsx),
            file_service: FileService::new(),
            pipeline: SymbolPipeline::new(Language::Typescript.id()),
        })
    }
}

#[async_trait]
impl Chunker for TsChunker {
    async fn split(&self, path: &Path, relative_path: &str) -> anyhow::Result<Vec<Chunk>> {
        let source = self.file_service.read_file_to_string(path).await?;

        let tree = {
            let mut parser = if is_tsx(path) {
                self.parser_tsx.lock()
            } else {
                self.parser_ts.lock()
            };
            parser
                .parse(&source, None)
                .ok_or_else(|| anyhow::anyhow!("tree-sitter parse returned None"))?
        };

        Ok(self.pipeline.split_file_to_chunks(
            &tree,
            &source,
            relative_path,
            SymbolKind::from_node_kind,
        ))
    }
}

fn is_tsx(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("tsx"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::document_chunker::chunker::Chunker;

    fn temp_ts_file(name: &str, source: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("ts_chunker_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("mkdir");
        let path = dir.join(name);
        fs::write(&path, source).expect("write");
        (path, dir)
    }

    #[tokio::test]
    async fn split_parses_common_symbol_kinds() {
        let src = r#"export interface Iface {
  x: number;
}
export enum En {
  A,
  B,
}
export type StructLike = { a: number };
export function fnTop(): void {}
export class Empty {}
export class WithMethod {
  methodInClass(): void {}
}
"#;
        let (path, dir) = temp_ts_file("sample.ts", src);
        let chunker = TsChunker::new().expect("TsChunker::new");
        let chunks = chunker.split(&path, "sample.ts").await.expect("split");
        assert!(!chunks.is_empty());

        let chunks_gt = vec![
            "Iface",
            "En",
            "StructLike",
            "fnTop",
            "Empty",
            "WithMethod",
            "methodInClass",
        ];
        for (chunk, chunk_gt) in chunks.iter().zip(chunks_gt.iter()) {
            assert_eq!(chunk.embedding_content, *chunk_gt);
        }

        fs::remove_dir_all(&dir).ok();
    }
}
