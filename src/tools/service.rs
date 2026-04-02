use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use clap::ValueEnum;
use ort::session::builder::GraphOptimizationLevel;
use serde::Serialize;

use crate::{
    common::data::{IndexType, QueryResult},
    embedding::utils::{EmbeddingModelType, EmbeddingOptions, OnnxRuntimeConfig},
    index::utils::{CancelToken, IndexProgress, ProgressReporter, SimpleCancelToken},
    manager::SemanticSearchManager,
    metrics::{data::IndexMetrics, profiler::IndexProfiler},
    storage::manager::StorageOptions,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ModelTypeArg {
    Baize,
    Veso,
}

impl From<ModelTypeArg> for EmbeddingModelType {
    fn from(value: ModelTypeArg) -> Self {
        match value {
            ModelTypeArg::Baize => EmbeddingModelType::Baize,
            ModelTypeArg::Veso => EmbeddingModelType::Veso,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum LayerSelector {
    File,
    Symbol,
    Content,
    All,
}

impl LayerSelector {
    pub fn to_layers(self) -> Vec<IndexType> {
        match self {
            Self::File => vec![IndexType::File],
            Self::Symbol => vec![IndexType::Symbol],
            Self::Content => vec![IndexType::Content],
            Self::All => vec![IndexType::File, IndexType::Symbol],
        }
    }
}

pub struct ResolvedConfig {
    pub project: PathBuf,
    pub storage_options: StorageOptions,
    pub onnx_runtime_config: OnnxRuntimeConfig,
    pub embedding_options: EmbeddingOptions,
    pub output: OutputFormat,
}

#[derive(Debug, Clone)]
pub struct IndexRequest {
    pub project: String,
    pub layer: LayerSelector,
}

#[derive(Debug, Clone)]
pub struct StartIndexRequest {
    pub project: String,
    pub layer: LayerSelector,
}

#[derive(Debug, Clone)]
pub struct StopIndexRequest {
    pub project: String,
}

#[derive(Debug, Clone)]
pub struct IndexProgressRequest {
    pub project: String,
}

#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub project: String,
    pub query: String,
    pub layer: LayerSelector,
    pub limit: usize,
    pub threshold: f32,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexRunStatus {
    NotStarted,
    Running,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Serialize)]
pub struct StartIndexResponse {
    pub project: String,
    pub layers: Vec<String>,
    pub status: IndexRunStatus,
    pub usage_hint: String,
}

#[derive(Debug, Serialize)]
pub struct StopIndexResponse {
    pub project: String,
    pub status: IndexRunStatus,
    pub usage_hint: String,
}

#[derive(Debug, Serialize)]
pub struct IndexProgressResponse {
    pub project: String,
    pub status: IndexRunStatus,
    pub progress: IndexProgress,
    pub last_error: Option<String>,
    pub usage_hint: String,
}

#[async_trait]
pub trait SemanticSearchBackend: Send + Sync {
    async fn index_layers(&self, layers: &[IndexType]) -> anyhow::Result<IndexMetrics>;
    async fn start_indexing(&self, layers: &[IndexType]) -> anyhow::Result<IndexRunStatus>;
    async fn index_progress(&self) -> anyhow::Result<IndexProgressResponse>;
    async fn stop_indexing(&self) -> anyhow::Result<IndexRunStatus>;
    async fn search_layers(
        &self,
        query: &str,
        limit: usize,
        threshold: f32,
        layers: &[IndexType],
        paths: Vec<String>,
    ) -> anyhow::Result<Vec<QueryResult>>;
}

pub struct ManagerBackend {
    manager: Arc<SemanticSearchManager>,
    session: Arc<Mutex<IndexSession>>,
    project: String,
}

struct IndexSession {
    status: IndexRunStatus,
    progress: IndexProgress,
    last_error: Option<String>,
    cancel_token: Arc<SimpleCancelToken>,
}

impl Default for IndexSession {
    fn default() -> Self {
        Self {
            status: IndexRunStatus::NotStarted,
            progress: IndexProgress {
                total_file_count: 0,
                total_symbol_count: 0,
                handled_file_count: 0,
                handled_symbol_count: 0,
            },
            last_error: None,
            cancel_token: Arc::new(SimpleCancelToken::new()),
        }
    }
}

struct SessionProgressReporter {
    session: Arc<Mutex<IndexSession>>,
}

impl ProgressReporter for SessionProgressReporter {
    fn on_progress(&self, progress: IndexProgress) {
        if let Ok(mut s) = self.session.lock() {
            s.progress = progress;
        }
    }

    fn on_completed(&self) {
        if let Ok(mut s) = self.session.lock() {
            s.status = IndexRunStatus::Completed;
        }
    }

    fn on_error(&self, error: String) {
        if let Ok(mut s) = self.session.lock() {
            s.status = IndexRunStatus::Error;
            s.last_error = Some(error);
        }
    }
}

impl ManagerBackend {
    pub async fn new(config: &ResolvedConfig) -> anyhow::Result<Self> {
        // This backend is used from binaries that already run inside a Tokio runtime.
        // Creating and dropping a nested runtime can panic during shutdown.
        let handle = tokio::runtime::Handle::current();
        let manager = SemanticSearchManager::new(
            config.storage_options.clone(),
            OnnxRuntimeConfig {
                runtime_path: config.onnx_runtime_config.runtime_path.clone(),
                intra_threads: config.onnx_runtime_config.intra_threads,
                optimization_level: GraphOptimizationLevel::Level1,
            },
            config.embedding_options.clone(),
            &config.project,
            handle.clone(),
        )
        .await?;
        Ok(Self {
            manager: Arc::new(manager),
            session: Arc::new(Mutex::new(IndexSession::default())),
            project: config.project.to_string_lossy().to_string(),
        })
    }
}

#[async_trait]
impl SemanticSearchBackend for ManagerBackend {
    async fn index_layers(&self, layers: &[IndexType]) -> anyhow::Result<IndexMetrics> {
        let profiler = Arc::new(IndexProfiler::new(self.manager.storage_options()));
        prepare_profiler_for_selected_layers(&profiler, layers);

        let progress_reporter: Arc<dyn ProgressReporter> = Arc::new(SilentProgressReporter);
        let cancel_token: Arc<dyn CancelToken> = Arc::new(SimpleCancelToken::new());

        for layer in layers {
            self.manager
                .index_layer(
                    *layer,
                    profiler.clone(),
                    progress_reporter.clone(),
                    cancel_token.clone(),
                )
                .await?;
        }

        Ok(profiler.stop_profiler())
    }

    async fn start_indexing(&self, layers: &[IndexType]) -> anyhow::Result<IndexRunStatus> {
        {
            let mut s = self.session.lock().expect("index session mutex poisoned");
            *s = IndexSession {
                status: IndexRunStatus::Running,
                progress: IndexProgress {
                    total_file_count: 0,
                    total_symbol_count: 0,
                    handled_file_count: 0,
                    handled_symbol_count: 0,
                },
                last_error: None,
                cancel_token: Arc::new(SimpleCancelToken::new()),
            };
        }

        let progress_reporter: Arc<dyn ProgressReporter> =
            Arc::new(SessionProgressReporter { session: self.session.clone() });
        let cancel_token: Arc<dyn CancelToken> = {
            let s = self.session.lock().expect("index session mutex poisoned");
            s.cancel_token.clone()
        };

        let profiler = Arc::new(IndexProfiler::new(self.manager.storage_options()));
        prepare_profiler_for_selected_layers(&profiler, layers);

        let remaining = Arc::new(std::sync::atomic::AtomicUsize::new(layers.len()));
        for layer in layers.iter().copied() {
            let manager = self.manager.clone();
            let profiler_clone = profiler.clone();
            let reporter_clone = progress_reporter.clone();
            let cancel_clone = cancel_token.clone();
            let remaining_clone = remaining.clone();
            let session_clone = self.session.clone();
            tokio::spawn(async move {
                let result = manager
                    .index_layer(layer, profiler_clone, reporter_clone.clone(), cancel_clone)
                    .await;
                match result {
                    Ok(()) => {
                        if remaining_clone.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) == 1 {
                            reporter_clone.on_completed();
                        }
                    }
                    Err(e) => {
                        reporter_clone.on_error(e.to_string());
                    }
                }

                // If cancelled, reflect it (best-effort).
                if let Ok(mut s) = session_clone.lock() {
                    if s.cancel_token.is_cancelled() && s.status == IndexRunStatus::Running {
                        s.status = IndexRunStatus::Cancelled;
                    }
                }
            });
        }

        Ok(IndexRunStatus::Running)
    }

    async fn index_progress(&self) -> anyhow::Result<IndexProgressResponse> {
        let guard = self.session.lock().expect("index session mutex poisoned");
        Ok(IndexProgressResponse {
            project: self.project.clone(),
            status: guard.status,
            progress: IndexProgress {
                total_file_count: guard.progress.total_file_count,
                total_symbol_count: guard.progress.total_symbol_count,
                handled_file_count: guard.progress.handled_file_count,
                handled_symbol_count: guard.progress.handled_symbol_count,
            },
            last_error: guard.last_error.clone(),
            usage_hint: "Use /index_progress to poll; use /stop_index to cancel; use /search anytime (may be partial while running).".to_string(),
        })
    }

    async fn stop_indexing(&self) -> anyhow::Result<IndexRunStatus> {
        let mut guard = self.session.lock().expect("index session mutex poisoned");
        guard.cancel_token.cancel();
        if guard.status == IndexRunStatus::Running {
            guard.status = IndexRunStatus::Cancelled;
        }
        Ok(guard.status)
    }

    async fn search_layers(
        &self,
        query: &str,
        limit: usize,
        threshold: f32,
        layers: &[IndexType],
        paths: Vec<String>,
    ) -> anyhow::Result<Vec<QueryResult>> {
        let mut merged_results: Vec<QueryResult> = Vec::new();
        for layer in layers {
            let mut results = self.manager
                .search(query, limit, threshold, *layer, paths.clone())
                .await?;
            merged_results.append(&mut results);
        }
        Ok(merged_results)
    }
}

fn prepare_profiler_for_selected_layers(profiler: &Arc<IndexProfiler>, layers: &[IndexType]) {
    if !layers.contains(&IndexType::File) {
        profiler.finish_layer(IndexType::File);
    }
    if !layers.contains(&IndexType::Symbol) {
        profiler.finish_layer(IndexType::Symbol);
    }
}

struct SilentProgressReporter;

impl ProgressReporter for SilentProgressReporter {
    fn on_progress(&self, _progress: crate::index::utils::IndexProgress) {}
    fn on_completed(&self) {}
    fn on_error(&self, _error: String) {}
}

pub struct CommandHandler<B> {
    backend: B,
}

impl<B> CommandHandler<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
}

impl<B: SemanticSearchBackend> CommandHandler<B> {
    pub async fn execute_index(&self, request: IndexRequest) -> anyhow::Result<CommandResponse> {
        let layers = request.layer.to_layers();
        let metrics = self.backend.index_layers(&layers).await?;
        Ok(CommandResponse::Index(IndexResponse {
            project: request.project,
            layers: layers.iter().map(ToString::to_string).collect(),
            metrics,
            usage_hint: "Run /search <query> after indexing; searching during indexing returns whatever has already been indexed.".to_string(),
        }))
    }

    pub async fn execute_start_index(
        &self,
        request: StartIndexRequest,
    ) -> anyhow::Result<CommandResponse> {
        let layers = request.layer.to_layers();
        let status = self.backend.start_indexing(&layers).await?;
        Ok(CommandResponse::StartIndex(StartIndexResponse {
            project: request.project,
            layers: layers.iter().map(ToString::to_string).collect(),
            status,
            usage_hint: "Use /index_progress to check progress; /stop_index to cancel; /search is available anytime.".to_string(),
        }))
    }

    pub async fn execute_index_progress(
        &self,
        _request: IndexProgressRequest,
    ) -> anyhow::Result<CommandResponse> {
        let response = self.backend.index_progress().await?;
        Ok(CommandResponse::IndexProgress(response))
    }

    pub async fn execute_stop_index(&self, request: StopIndexRequest) -> anyhow::Result<CommandResponse> {
        let status = self.backend.stop_indexing().await?;
        Ok(CommandResponse::StopIndex(StopIndexResponse {
            project: request.project,
            status,
            usage_hint: "You can restart with /start_index. Searching uses whatever has been indexed so far.".to_string(),
        }))
    }

    pub async fn execute_search(&self, request: SearchRequest) -> anyhow::Result<CommandResponse> {
        let layers = request.layer.to_layers();

        let mut results = self
            .backend
            .search_layers(&request.query, request.limit, request.threshold, &layers, request.paths.clone())
            .await?;

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(request.limit);

        let layer_label = match request.layer {
            LayerSelector::File => "file",
            LayerSelector::Symbol => "symbol",
            LayerSelector::Content => "content",
            LayerSelector::All => "all",
        };

        Ok(CommandResponse::Search(SearchResponse {
            project: request.project,
            query: request.query,
            layer: layer_label.to_string(),
            limit: request.limit,
            threshold: request.threshold,
            usage_hint: "If results are missing, run /index first or continue searching while indexing is still in progress.".to_string(),
            results: results
                .into_iter()
                .map(|result| SearchResultView {
                    score: result.score,
                    layer: result.info.layer.to_string(),
                    file_path: result.info.file_path,
                    lang: result.info.lang,
                    range: result.info.range.map(RangeView::from),
                    content: result.info.content,
                })
                .collect(),
        }))
    }
}

pub enum CommandResponse {
    Index(IndexResponse),
    StartIndex(StartIndexResponse),
    IndexProgress(IndexProgressResponse),
    StopIndex(StopIndexResponse),
    Search(SearchResponse),
}

impl CommandResponse {
    pub fn render(&self, format: OutputFormat) -> anyhow::Result<String> {
        match format {
            OutputFormat::Json => match self {
                Self::Index(response) => serde_json::to_string_pretty(response).map_err(Into::into),
                Self::StartIndex(response) => serde_json::to_string_pretty(response).map_err(Into::into),
                Self::IndexProgress(response) => serde_json::to_string_pretty(response).map_err(Into::into),
                Self::StopIndex(response) => serde_json::to_string_pretty(response).map_err(Into::into),
                Self::Search(response) => serde_json::to_string_pretty(response).map_err(Into::into),
            },
            OutputFormat::Text => Ok(match self {
                Self::Index(response) => response.render_text(),
                Self::StartIndex(response) => format!(
                    "Index started\nproject: {}\nlayers: {}\nstatus: {:?}\nnext: {}",
                    response.project,
                    response.layers.join(", "),
                    response.status,
                    response.usage_hint
                ),
                Self::IndexProgress(response) => format!(
                    "Index progress\nproject: {}\nstatus: {:?}\nhandled: {} files, {} symbols\ntotal: {} files, {} symbols\nnext: {}",
                    response.project,
                    response.status,
                    response.progress.handled_file_count,
                    response.progress.handled_symbol_count,
                    response.progress.total_file_count,
                    response.progress.total_symbol_count,
                    response.usage_hint
                ),
                Self::StopIndex(response) => format!(
                    "Index stopped\nproject: {}\nstatus: {:?}\nnext: {}",
                    response.project,
                    response.status,
                    response.usage_hint
                ),
                Self::Search(response) => response.render_text(),
            }),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IndexResponse {
    pub project: String,
    pub layers: Vec<String>,
    pub metrics: IndexMetrics,
    pub usage_hint: String,
}

impl IndexResponse {
    fn render_text(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(&mut out, "Index completed");
        let _ = writeln!(&mut out, "project: {}", self.project);
        let _ = writeln!(&mut out, "layers: {}", self.layers.join(", "));
        let _ = writeln!(
            &mut out,
            "handled: {} files, {} symbols",
            self.metrics.handled_file_count, self.metrics.handled_symbol_count
        );
        let _ = writeln!(
            &mut out,
            "duration: {:.2}s",
            self.metrics.total_time.as_secs_f64()
        );
        let _ = writeln!(&mut out, "next: {}", self.usage_hint);
        out.trim_end().to_string()
    }
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub project: String,
    pub query: String,
    pub layer: String,
    pub limit: usize,
    pub threshold: f32,
    pub results: Vec<SearchResultView>,
    pub usage_hint: String,
}

impl SearchResponse {
    fn render_text(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(&mut out, "Search results");
        let _ = writeln!(&mut out, "project: {}", self.project);
        let _ = writeln!(&mut out, "query: {}", self.query);
        let _ = writeln!(&mut out, "layer: {}", self.layer);
        if self.results.is_empty() {
            let _ = writeln!(&mut out, "results: none");
        } else {
            let _ = writeln!(&mut out, "results:");
            for result in &self.results {
                let _ = writeln!(
                    &mut out,
                    "- {:.3} {} {}",
                    result.score, result.file_path, result.layer
                );
                if let Some(range) = &result.range {
                    let _ = writeln!(
                        &mut out,
                        "  range: {}:{}-{}:{}",
                        range.start_line,
                        range.start_character,
                        range.end_line,
                        range.end_character
                    );
                }
                if let Some(content) = &result.content {
                    let snippet = content.lines().next().unwrap_or("");
                    let _ = writeln!(&mut out, "  content: {}", snippet);
                }
            }
        }
        let _ = writeln!(&mut out, "next: {}", self.usage_hint);
        out.trim_end().to_string()
    }
}

#[derive(Debug, Serialize)]
pub struct SearchResultView {
    pub score: f32,
    pub layer: String,
    pub file_path: String,
    pub lang: String,
    pub range: Option<RangeView>,
    pub content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RangeView {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

impl From<lsp_types::Range> for RangeView {
    fn from(value: lsp_types::Range) -> Self {
        Self {
            start_line: value.start.line,
            start_character: value.start.character,
            end_line: value.end.line,
            end_character: value.end.character,
        }
    }
}

pub fn validate_project_exists(path: &Path) -> anyhow::Result<()> {
    if path.exists() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "project path does not exist: {}",
            path.display()
        ))
    }
}
