use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand};
use ort::session::builder::GraphOptimizationLevel;

use crate::{
    common::logger::init_logger,
    embedding::utils::{EmbeddingOptions, OnnxRuntimeConfig},
    storage::manager::StorageOptions,
    tools::service::{
        CommandHandler, IndexRequest, LayerSelector, ManagerBackend, ModelTypeArg, OutputFormat,
        ResolvedConfig, SearchRequest, validate_project_exists,
    },
};

#[derive(Debug, Parser)]
#[command(name = "semantic-search")]
#[command(about = "Local semantic indexing and search for source repositories")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Build or refresh semantic indexes for a repository.
    Index(IndexCommand),
    /// Search previously indexed repository content.
    Search(SearchCommand),
}

#[derive(Debug, Clone, Parser)]
pub struct SharedArgs {
    /// Repository root to index/search.
    #[arg(long, env = "SEMANTIC_SEARCH_PROJECT")]
    pub project: PathBuf,

    /// SQLite index metadata path.
    #[arg(long, env = "SEMANTIC_SEARCH_INDEX_DB")]
    pub index_db_path: Option<PathBuf>,

    /// LanceDB vector storage path.
    #[arg(long, env = "SEMANTIC_SEARCH_VECTOR_DB")]
    pub vector_db_path: Option<PathBuf>,

    /// ONNX runtime dylib / shared library path.
    #[arg(long, env = "SEMANTIC_SEARCH_ONNX_RUNTIME")]
    pub onnx_runtime_path: Option<PathBuf>,

    /// Embedding model file path.
    #[arg(long, env = "SEMANTIC_SEARCH_MODEL")]
    pub model_path: Option<PathBuf>,

    /// Tokenizer json path.
    #[arg(long, env = "SEMANTIC_SEARCH_TOKENIZER")]
    pub tokenizer_path: Option<PathBuf>,

    /// Embedding model kind.
    #[arg(long, env = "SEMANTIC_SEARCH_MODEL_TYPE", value_enum, default_value_t = ModelTypeArg::Veso)]
    pub model_type: ModelTypeArg,

    /// Embedding dimension.
    #[arg(long, env = "SEMANTIC_SEARCH_EMBEDDING_DIM", default_value_t = 768)]
    pub dim: usize,

    /// Embedding batch size.
    #[arg(long, env = "SEMANTIC_SEARCH_BATCH_SIZE", default_value_t = 32)]
    pub batch_size: usize,

    /// Worker threads used during indexing/search.
    #[arg(long, env = "SEMANTIC_SEARCH_THREADS", default_value_t = 1)]
    pub num_threads: usize,

    /// ONNX intra-op threads.
    #[arg(long, env = "SEMANTIC_SEARCH_INTRA_THREADS", default_value_t = 1)]
    pub intra_threads: usize,

    /// Log file path.
    #[arg(long, env = "SEMANTIC_SEARCH_LOG_PATH")]
    pub log_path: Option<PathBuf>,

    /// Logger level.
    #[arg(long, env = "SEMANTIC_SEARCH_LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    /// Human-readable text or JSON output.
    #[arg(long, env = "SEMANTIC_SEARCH_OUTPUT", value_enum, default_value_t = OutputFormat::Text)]
    pub output: OutputFormat,
}

#[derive(Debug, Clone, Parser)]
pub struct IndexCommand {
    #[command(flatten)]
    pub shared: SharedArgs,

    /// Index the file layer, symbol layer, or both.
    #[arg(long, value_enum, default_value_t = LayerSelector::All)]
    pub layer: LayerSelector,
}

#[derive(Debug, Clone, Parser)]
pub struct SearchCommand {
    #[command(flatten)]
    pub shared: SharedArgs,

    /// Search query text.
    #[arg(long)]
    pub query: String,

    /// Search within one or multiple index layers (use `all` for file+symbol).
    #[arg(long, value_enum, default_value_t = LayerSelector::Symbol)]
    pub layer: LayerSelector,

    /// Maximum number of hits to return.
    #[arg(long, default_value_t = 10)]
    pub limit: usize,

    /// Minimum similarity score.
    #[arg(long, default_value_t = 0.5)]
    pub threshold: f32,

    /// Restrict search to one or more relative paths.
    #[arg(long = "path")]
    pub paths: Vec<String>,
}

impl SharedArgs {
    pub fn resolve(&self) -> anyhow::Result<ResolvedConfig> {
        validate_project_exists(&self.project)?;
        let default_paths = crate::resources::data_dir::platform_default_data_paths()?;

        let index_db_path = self
            .index_db_path
            .clone()
            .unwrap_or_else(|| default_paths.index_db_path.clone());
        let vector_db_path = self
            .vector_db_path
            .clone()
            .unwrap_or_else(|| default_paths.vector_db_path.clone());
        let log_path = self
            .log_path
            .clone()
            .unwrap_or_else(|| default_paths.log_path.clone());

        if let Some(parent) = index_db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(&vector_db_path)?;
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }

        init_logger(
            log_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("log path must be valid UTF-8"))?,
            &self.log_level,
        )?;

        let model_type: crate::embedding::utils::EmbeddingModelType = self.model_type.into();
        let runtime_path = match &self.onnx_runtime_path {
            Some(p) => p.to_string_lossy().to_string(),
            None => crate::resources::paths::default_onnxruntime_path()?.to_string_lossy().to_string(),
        };

        let model_path = match &self.model_path {
            Some(p) => p.clone(),
            None => crate::resources::paths::default_embedding_model_path(model_type.clone())?,
        };

        let tokenizer_path = match &self.tokenizer_path {
            Some(p) => p.clone(),
            None => crate::resources::paths::default_tokenizer_path(model_type)?,
        };

        Ok(ResolvedConfig {
            project: self.project.clone(),
            storage_options: StorageOptions {
                index_db_path,
                vector_db_path,
            },
            onnx_runtime_config: OnnxRuntimeConfig {
                runtime_path,
                intra_threads: self.intra_threads,
                optimization_level: GraphOptimizationLevel::Level1,
            },
            embedding_options: EmbeddingOptions {
                model_type: self.model_type.into(),
                model_path,
                tokenizer_path,
                dim: self.dim,
                batch_size: self.batch_size,
                num_threads: self.num_threads,
            },
            output: self.output,
        })
    }
}

pub async fn run_cli(cli: Cli) -> anyhow::Result<String> {
    match cli.command {
        Commands::Index(command) => {
            let config = command.shared.resolve()?;
            let backend = ManagerBackend::new(&config).await?;
            let handler = CommandHandler::new(backend);
            let response = handler
                .execute_index(IndexRequest {
                    project: command.shared.project.to_string_lossy().to_string(),
                    layer: command.layer,
                })
                .await?;
            response.render(config.output)
        }
        Commands::Search(command) => {
            let config = command.shared.resolve()?;
            let backend = ManagerBackend::new(&config).await?;
            let handler = CommandHandler::new(backend);
            let response = handler
                .execute_search(SearchRequest {
                    project: command.shared.project.to_string_lossy().to_string(),
                    query: command.query,
                    layer: command.layer,
                    limit: command.limit,
                    threshold: command.threshold,
                    paths: command.paths,
                })
                .await?;
            response.render(config.output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use lsp_types::{Position, Range};
    use crate::{
        common::data::{IndexType, QueryResult},
        metrics::data::IndexMetrics,
        resources::data_dir,
        tools::service::{SearchRequest, SemanticSearchBackend},
    };

    struct MockBackend {
        metrics: IndexMetrics,
        results: Vec<QueryResult>,
    }

    #[async_trait]
    impl SemanticSearchBackend for MockBackend {
        async fn index_layers(&self, _layers: &[IndexType]) -> anyhow::Result<IndexMetrics> {
            Ok(self.metrics.clone())
        }

        async fn start_indexing(
            &self,
            _layers: &[IndexType],
        ) -> anyhow::Result<crate::tools::service::IndexRunStatus> {
            Ok(crate::tools::service::IndexRunStatus::Running)
        }

        async fn index_progress(
            &self,
        ) -> anyhow::Result<crate::tools::service::IndexProgressResponse> {
            Ok(crate::tools::service::IndexProgressResponse {
                project: "/tmp/repo".to_string(),
                status: crate::tools::service::IndexRunStatus::Running,
                progress: crate::index::utils::IndexProgress::default(),
                last_error: None,
                usage_hint: "hint".to_string(),
            })
        }

        async fn stop_indexing(
            &self,
        ) -> anyhow::Result<crate::tools::service::IndexRunStatus> {
            Ok(crate::tools::service::IndexRunStatus::Cancelled)
        }

        async fn search_layers(
            &self,
            _query: &str,
            _limit: usize,
            _threshold: f32,
            _layers: &[IndexType],
            _paths: Vec<String>,
        ) -> anyhow::Result<Vec<QueryResult>> {
            Ok(self.results.clone())
        }
    }

    fn shared_args() -> SharedArgs {
        SharedArgs {
            project: PathBuf::from("/tmp/repo"),
            index_db_path: Some(PathBuf::from("/tmp/index.db")),
            vector_db_path: Some(PathBuf::from("/tmp/lancedb")),
            onnx_runtime_path: Some(PathBuf::from("/tmp/onnxruntime.dylib")),
            model_path: Some(PathBuf::from("/tmp/model.onnx")),
            tokenizer_path: Some(PathBuf::from("/tmp/tokenizer.json")),
            model_type: ModelTypeArg::Veso,
            dim: 768,
            batch_size: 32,
            num_threads: 1,
            intra_threads: 1,
            log_path: None,
            log_level: "info".to_string(),
            output: OutputFormat::Json,
        }
    }

    #[test]
    fn default_data_paths_match_contract() {
        let base = PathBuf::from("/tmp/data_dir");
        let paths = data_dir::default_paths_under(&base);
        assert_eq!(paths.index_db_path, base.join("semantic_search/index.db"));
        assert_eq!(paths.vector_db_path, base.join("semantic_search/vectordb"));
        assert_eq!(paths.log_path, base.join("semantic_search/running.log"));
    }

    #[test]
    fn parse_index_command_shape() {
        let cli = Cli::try_parse_from([
            "semantic-search",
            "index",
            "--project",
            "/repo",
            "--index-db-path",
            "/repo/.semantic/index.db",
            "--vector-db-path",
            "/repo/.semantic/lancedb",
            "--onnx-runtime-path",
            "/models/onnxruntime.dylib",
            "--model-path",
            "/models/model.onnx",
            "--tokenizer-path",
            "/models/tokenizer.json",
            "--layer",
            "all",
        ])
        .expect("parse index command");

        match cli.command {
            Commands::Index(command) => {
                assert_eq!(command.layer, LayerSelector::All);
                assert_eq!(command.shared.project, PathBuf::from("/repo"));
            }
            _ => panic!("expected index command"),
        }
    }

    #[test]
    fn parse_search_command_shape() {
        let cli = Cli::try_parse_from([
            "semantic-search",
            "search",
            "--project",
            "/repo",
            "--index-db-path",
            "/repo/.semantic/index.db",
            "--vector-db-path",
            "/repo/.semantic/lancedb",
            "--onnx-runtime-path",
            "/models/onnxruntime.dylib",
            "--model-path",
            "/models/model.onnx",
            "--tokenizer-path",
            "/models/tokenizer.json",
            "--query",
            "audio player state",
            "--layer",
            "symbol",
            "--limit",
            "8",
            "--threshold",
            "0.42",
            "--path",
            "src/a.ts",
            "--path",
            "src/b.ts",
            "--output",
            "json",
        ])
        .expect("parse search command");

        match cli.command {
            Commands::Search(command) => {
                assert_eq!(command.query, "audio player state");
                assert_eq!(command.layer, LayerSelector::Symbol);
                assert_eq!(command.limit, 8);
                assert!((command.threshold - 0.42).abs() < f32::EPSILON);
                assert_eq!(
                    command.paths,
                    vec!["src/a.ts".to_string(), "src/b.ts".to_string()]
                );
                assert_eq!(command.shared.output, OutputFormat::Json);
            }
            _ => panic!("expected search command"),
        }
    }

    #[tokio::test]
    async fn execute_search_renders_json_output() {
        let backend = MockBackend {
            metrics: IndexMetrics::default(),
            results: vec![QueryResult {
                score: 0.93,
                info: crate::common::data::ChunkInfo {
                    layer: IndexType::Symbol,
                    lang: "typescript".to_string(),
                    file_path: "src/a.ts".to_string(),
                    content: Some("class A {}".to_string()),
                    range: Some(Range {
                        start: Position {
                            line: 1,
                            character: 2,
                        },
                        end: Position {
                            line: 3,
                            character: 4,
                        },
                    }),
                },
            }],
        };
        let handler = CommandHandler::new(backend);

        let response = handler
            .execute_search(SearchRequest {
                project: shared_args().project.to_string_lossy().to_string(),
                query: "class a".to_string(),
                layer: LayerSelector::Symbol,
                limit: 5,
                threshold: 0.3,
                paths: vec!["src/a.ts".to_string()],
            })
            .await
            .expect("execute search");

        let output = response.render(OutputFormat::Json).expect("render json");
        assert!(output.contains("\"query\": \"class a\""));
        assert!(output.contains("\"file_path\": \"src/a.ts\""));
        assert!(output.contains("\"start_line\": 1"));
    }

    #[tokio::test]
    async fn execute_index_renders_text_hint() {
        let backend = MockBackend {
            metrics: IndexMetrics {
                handled_file_count: 3,
                handled_symbol_count: 8,
                ..Default::default()
            },
            results: vec![],
        };
        let handler = CommandHandler::new(backend);

        let response = handler
            .execute_index(IndexRequest {
                project: shared_args().project.to_string_lossy().to_string(),
                layer: LayerSelector::All,
            })
            .await
            .expect("execute index");

        let output = response.render(OutputFormat::Text).expect("render text");
        assert!(output.contains("Index completed"));
        assert!(output.contains("handled: 3 files, 8 symbols"));
        assert!(output.contains("Run /search <query> after indexing"));
    }
}
