use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;

use lsp_types::Range;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone)]
pub struct Project {
    pub id: i64,
    pub root_path: PathBuf,
    pub embedding_model: String,
    pub hash: String,
    pub index_finished_time: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct IndexStatus {
    pub project_id: i64,
    // relative path
    pub file_path: String,
    pub layer: IndexType,
    pub file_hash: String,
    pub mtime: u64,
    pub ctime: u64,
    pub size: u64,
    pub indexed_at: u64,
}

impl IndexStatus {
    pub fn is_changed(&self, other: &Self) -> bool {
        if self.size != other.size {
            return true;
        }
        if self.mtime == other.mtime && self.ctime == other.ctime {
            return false;
        }

        self.file_hash != other.file_hash
    }
}

pub struct IndexDiff<'a> {
    pub deleted: Vec<&'a IndexStatus>,
    pub new: Vec<&'a IndexStatus>,
    pub updated: Vec<&'a IndexStatus>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexType {
    #[default]
    File,
    Symbol,
    Content,
}

impl Display for IndexType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexType::File => write!(f, "file"),
            IndexType::Symbol => write!(f, "symbol"),
            IndexType::Content => write!(f, "content"),
        }
    }
}

impl FromStr for IndexType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "file" => Ok(IndexType::File),
            "symbol" => Ok(IndexType::Symbol),
            "content" => Ok(IndexType::Content),
            _ => Err(anyhow::anyhow!("invalid index type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ChunkInfo {
    pub layer: IndexType,
    pub lang: String,
    /// relative path
    pub file_path: String,
    pub content: Option<String>,
    pub range: Option<Range>,
}

#[derive(Clone, Debug, Default)]
pub struct Chunk {
    pub embedding_content: String,
    pub info: ChunkInfo,
    pub embedding: Vec<f32>,
    pub is_last: bool,
}

pub enum ChunkMsg {
    Chunk(Chunk),
    FileStart { file_path: String },
    FileEnd { file_status: IndexStatus },
}

impl Debug for ChunkMsg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkMsg::Chunk(chunk) => write!(f, "Chunk({:?})", chunk),
            ChunkMsg::FileStart { file_path } => write!(f, "FileStart({})", file_path),
            ChunkMsg::FileEnd { file_status } => write!(f, "FileEnd({:?})", file_status),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub score: f32,
    pub info: ChunkInfo,
}
