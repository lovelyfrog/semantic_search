use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
};

use ort::session::builder::GraphOptimizationLevel;
use serde::Deserialize;
use tokenizers::Encoding;

#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingModelType {
    Baize,
    Veso,
}

impl Display for EmbeddingModelType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Baize => write!(f, "baize"),
            Self::Veso => write!(f, "veso"),
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct EmbeddingOptions {
    pub model_type: EmbeddingModelType,
    pub model_path: PathBuf,
    pub tokenizer_path: PathBuf,
    pub dim: usize,
    pub batch_size: usize,
    pub num_threads: usize,
}

pub struct OnnxRuntimeConfig {
    pub runtime_path: String,
    pub intra_threads: usize,
    pub optimization_level: GraphOptimizationLevel,
}

pub trait Embedder {
    fn encode(&self, text: &str) -> anyhow::Result<Encoding>;
    fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>>;
    fn batch_embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>>;
}
