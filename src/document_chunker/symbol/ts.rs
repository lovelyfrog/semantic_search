use std::path::Path;

type ByteRange = std::ops::Range<usize>;

use async_trait::async_trait;
use lsp_types::{Position, Range as LspRange};
use parking_lot::Mutex;
use tree_sitter::{Node, Parser};

use crate::common::FileService;
use crate::common::data::{Chunk, ChunkInfo, IndexType};
use crate::document_chunker::chunker::Chunker;
use crate::language::language::Language;

pub struct TsChunker {
    parser_ts: Mutex<Parser>,
    parser_tsx: Mutex<Parser>,
    file_service: FileService,
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

        let mut symbols: Vec<(ByteRange, TsSymbolKind, String)> = Vec::new();
        collect_symbols(tree.root_node(), &source, &mut symbols);

        symbols.retain(|(r, _, _)| !r.is_empty());
        symbols.sort_by_key(|(r, _, _)| r.end - r.start);
        let symbols = pick_non_overlapping(symbols);

        let chunk_count = symbols.len();
        let mut chunks = Vec::with_capacity(chunk_count);
        let lang_id = Language::Typescript.id();

        for (idx, (range, kind, name)) in symbols.into_iter().enumerate() {
            let content = source.get(range.clone()).unwrap_or("").to_string();
            let embedding_content = format!("[{}] {}\n{}", kind.as_label(), name, content);
            chunks.push(Chunk {
                embedding_content,
                info: ChunkInfo {
                    layer: IndexType::Symbol,
                    lang: lang_id.clone(),
                    file_path: relative_path.to_string(),
                    content: Some(content),
                    range: Some(range_to_lsp(&source, &range)),
                },
                is_last: idx + 1 == chunk_count,
                ..Default::default()
            });
        }

        Ok(chunks)
    }
}

fn is_tsx(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("tsx"))
        .unwrap_or(false)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TsSymbolKind {
    Class,
    Interface,
    Function,
    Method,
    Enum,
    // TS has no native struct keyword; we map type aliases to struct-like chunks.
    Struct,
}

impl TsSymbolKind {
    fn as_label(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Function => "function",
            Self::Method => "method",
            Self::Enum => "enum",
            Self::Struct => "struct",
        }
    }

    fn from_node_kind(kind: &str) -> Option<Self> {
        match kind {
            "class_declaration" | "abstract_class_declaration" => Some(Self::Class),
            "interface_declaration" => Some(Self::Interface),
            "function_declaration" | "generator_function_declaration" => Some(Self::Function),
            "method_definition" => Some(Self::Method),
            "enum_declaration" => Some(Self::Enum),
            "type_alias_declaration" => Some(Self::Struct),
            _ => None,
        }
    }
}

fn collect_symbols(node: Node<'_>, source: &str, out: &mut Vec<(ByteRange, TsSymbolKind, String)>) {
    if let Some(kind) = TsSymbolKind::from_node_kind(node.kind()) {
        let name = node
            .child_by_field_name("name")
            .map(|n| source[n.byte_range()].to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "<anonymous>".to_string());
        out.push((node.start_byte()..node.end_byte(), kind, name));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_symbols(child, source, out);
    }
}

// Prefer narrower nodes for overlap (method over class).
fn pick_non_overlapping(
    mut sorted_by_len: Vec<(ByteRange, TsSymbolKind, String)>,
) -> Vec<(ByteRange, TsSymbolKind, String)> {
    let mut chosen: Vec<(ByteRange, TsSymbolKind, String)> = Vec::new();
    'next: for (range, kind, name) in sorted_by_len.drain(..) {
        for (existing, _, _) in &chosen {
            if ranges_overlap(&range, existing) {
                continue 'next;
            }
        }
        chosen.push((range, kind, name));
    }
    chosen.sort_by_key(|(range, _, _)| range.start);
    chosen
}

fn ranges_overlap(a: &ByteRange, b: &ByteRange) -> bool {
    a.start < b.end && b.start < a.end
}

fn range_to_lsp(source: &str, range: &ByteRange) -> LspRange {
    LspRange {
        start: byte_offset_to_position(source, range.start),
        end: byte_offset_to_position(source, range.end),
    }
}

fn byte_offset_to_position(text: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut col_utf16 = 0u32;
    let mut i = 0usize;
    for ch in text.chars() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col_utf16 = 0;
        } else {
            col_utf16 += ch.len_utf16() as u32;
        }
        i += ch.len_utf8();
    }
    Position {
        line,
        character: col_utf16,
    }
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

    fn first_line(s: &str) -> &str {
        s.lines().next().unwrap_or("")
    }

    fn position_to_offset(text: &str, pos: Position) -> usize {
        let mut line = 0u32;
        let mut col_utf16 = 0u32;
        let mut offset = 0usize;
        for ch in text.chars() {
            if line == pos.line && col_utf16 >= pos.character {
                break;
            }
            if ch == '\n' {
                line += 1;
                col_utf16 = 0;
            } else {
                col_utf16 += ch.len_utf16() as u32;
            }
            offset += ch.len_utf8();
        }
        offset
    }

    #[tokio::test]
    async fn split_sets_content_and_range() {
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

        let headers: Vec<&str> = chunks
            .iter()
            .map(|c| first_line(&c.embedding_content))
            .collect();
        assert!(headers.iter().any(|h| *h == "[interface] Iface"));
        assert!(headers.iter().any(|h| *h == "[enum] En"));
        assert!(headers.iter().any(|h| *h == "[struct] StructLike"));
        assert!(headers.iter().any(|h| *h == "[function] fnTop"));
        assert!(headers.iter().any(|h| *h == "[class] Empty"));
        assert!(headers.iter().any(|h| *h == "[method] methodInClass"));

        for chunk in &chunks {
            let content = chunk.info.content.as_ref().expect("content should be set");
            let range = chunk.info.range.as_ref().expect("range should be set");
            assert!(!content.is_empty());

            let start = position_to_offset(src, range.start);
            let end = position_to_offset(src, range.end);
            assert!(
                start <= end && end <= src.len(),
                "invalid range: {:?}",
                range
            );

            let slice = &src[start..end];
            assert_eq!(
                slice,
                content,
                "content should match source slice by range for {:?}",
                first_line(&chunk.embedding_content)
            );
        }

        fs::remove_dir_all(&dir).ok();
    }
}
