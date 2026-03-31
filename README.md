# vnext_semantic_search

**English** | [简体中文](README.zh-CN.md)

A Rust library for **semantic search** over codebases: chunk documents, embed with **ONNX Runtime**, persist metadata in **SQLite**, store vectors in **LanceDB**, and query with cosine similarity. Supports layered indexing (e.g. file / symbol / content), incremental updates via file hashes and mtimes, and async indexing workers.

## Features

- **Orchestration**: `SemanticSearchManager` wires storage, embedding, and `IndexManager`.
- **Storage**: SQLite for projects and per-file index status; LanceDB for chunk embeddings and ANN search.
- **Embedding**: `OnnxEmbedder` with batching and configurable graph optimization (with safe fallback for some FP16 models).
- **Chunking**: pluggable chunkers (e.g. whole-file, tree-sitter TypeScript symbols) via `ChunkerRegistry`.
- **Metrics**: index profiling and stage timers (`metrics` module).

## Requirements

- **Rust** toolchain compatible with `edition = "2024"`.
- **ONNX Runtime** shared library on the system or path expected by `ort` (see embedding tests / your deployment).

## Build

```bash
cargo build --release
cargo test
```

Format (without [cargo-make](https://github.com/sagiegurari/cargo-make)):

```bash
cargo fmt --all
```

Optional: install `cargo install cargo-make`, then `cargo make fmt-fix` / `cargo make clippy` using `Makefile.toml`.

## Layout

| Path | Role |
|------|------|
| `src/manager.rs` | `SemanticSearchManager` entry |
| `src/storage/` | `StorageManager`, SQLite `rdb/`, LanceDB `vector_db/` |
| `src/embedding/` | ONNX embedder and options |
| `src/index/` | indexing manager, worker, file checker |
| `src/document_chunker/` | chunkers and TS symbol parsing |
| `src/metrics/` | metrics data, profiler, timers |
| `docs/DESIGN.md` | architecture and data flow (detailed) |

## Documentation

- Design (English-friendly diagrams and tables): [`docs/DESIGN.md`](docs/DESIGN.md) — written in Chinese; use a translator or read code comments alongside.

## License

Add a `LICENSE` file at the repository root when you publish this project.
