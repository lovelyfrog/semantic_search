use std::path::PathBuf;
use std::sync::Arc;

use ort::session::builder::GraphOptimizationLevel;

use crate::common::logger::init_logger;
use crate::embedding::onnx::OnnxEmbedder;
use crate::embedding::utils::{EmbeddingModelType, EmbeddingOptions, OnnxRuntimeConfig};
use crate::index::utils::{
    CancelToken, ConsoleProgressReporter, ProgressReporter, SimpleCancelToken,
};
use crate::manager::SemanticSearchManager;
use crate::storage::manager::StorageOptions;

lazy_static::lazy_static! {
    static ref RUNTIME: tokio::runtime::Runtime = tokio::runtime::Runtime::new().unwrap();
}

pub fn temp_dir() -> PathBuf {
    PathBuf::from("/Users/vnext/Documents/zbq/ai/semantic_search_resources")
}

pub fn runtime_path() -> String {
    temp_dir()
        .join("models/onnxruntime.dylib")
        .to_string_lossy()
        .to_string()
}

pub fn load_onnx_runtime() -> anyhow::Result<()> {
    OnnxEmbedder::load_runtime(&runtime_path())
}

pub fn setup_embedder(
    intra_threads: usize,
    batch_size: usize,
    num_threads: usize,
) -> anyhow::Result<OnnxEmbedder> {
    let runtime_path = runtime_path();
    let config = OnnxRuntimeConfig {
        runtime_path,
        intra_threads,
        // Level3 may crash on some FP16 models (LayerNorm fusion); Level1 is enough for tests.
        optimization_level: GraphOptimizationLevel::Level1,
    };
    let embedding_options = EmbeddingOptions {
        model_type: EmbeddingModelType::Veso,
        model_path: temp_dir().join("models/model_fp16.onnx"),
        tokenizer_path: temp_dir().join("models/tokenizer.json"),
        dim: 768,
        batch_size,
        num_threads,
    };
    OnnxEmbedder::new(config, embedding_options)
}

pub fn init_log() {
    let log_path = temp_dir()
        .join("logs/test.log")
        .to_string_lossy()
        .to_string();
    let _ = init_logger(&log_path, "info");
}

pub fn options() -> (StorageOptions, OnnxRuntimeConfig, EmbeddingOptions) {
    let storage_options = StorageOptions {
        index_db_path: temp_dir().join("db/test.db"),
        vector_db_path: temp_dir().join("db/lancedb"),
    };
    let onnx_runtime_config = OnnxRuntimeConfig {
        runtime_path: runtime_path(),
        intra_threads: 1,
        optimization_level: GraphOptimizationLevel::Level1,
    };
    let embedding_options = EmbeddingOptions {
        model_type: EmbeddingModelType::Veso,
        model_path: temp_dir().join("models/model_fp16.onnx"),
        tokenizer_path: temp_dir().join("models/tokenizer.json"),
        dim: 768,
        batch_size: 32,
        num_threads: 1,
    };
    (storage_options, onnx_runtime_config, embedding_options)
}

pub fn tokenizer_path() -> PathBuf {
    temp_dir().join("models/tokenizer.json")
}

pub fn project_path() -> PathBuf {
    temp_dir().join("projects/hmosworld/commons")
}

pub async fn construct_manager() -> anyhow::Result<SemanticSearchManager> {
    let (storage_options, onnx_runtime_config, embedding_options) = options();
    let manager = SemanticSearchManager::new(
        storage_options,
        onnx_runtime_config,
        embedding_options,
        &project_path(),
        RUNTIME.handle().clone(),
    )
    .await?;
    Ok(manager)
}

pub fn index_reporter() -> Arc<dyn ProgressReporter> {
    Arc::new(ConsoleProgressReporter::new())
}

pub fn cancel_token() -> Arc<dyn CancelToken> {
    Arc::new(SimpleCancelToken::new())
}
