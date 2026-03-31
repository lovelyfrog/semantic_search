use std::path::Path;
use std::time::SystemTime;

use anyhow::Context;
use tokio::fs;

/// Async filesystem helpers for reading files and inspecting path metadata.
#[derive(Debug, Default, Clone, Copy)]
pub struct FileService;

impl FileService {
    pub const fn new() -> Self {
        Self
    }

    /// Reads the entire file into a byte vector.
    pub async fn read_file(&self, path: impl AsRef<Path>) -> anyhow::Result<Vec<u8>> {
        let path = path.as_ref();
        fs::read(path)
            .await
            .with_context(|| format!("failed to read file: {}", path.display()))
    }

    /// Reads the entire file as UTF-8.
    pub async fn read_file_to_string(&self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        let path = path.as_ref();
        fs::read_to_string(path)
            .await
            .with_context(|| format!("failed to read file as UTF-8: {}", path.display()))
    }

    /// Returns [`std::fs::Metadata`] for a path (regular file, directory, symlink target, etc.).
    pub async fn file_metadata(&self, path: impl AsRef<Path>) -> anyhow::Result<std::fs::Metadata> {
        let path = path.as_ref();
        fs::metadata(path)
            .await
            .with_context(|| format!("failed to stat path: {}", path.display()))
    }

    /// Same as [`Self::file_metadata`], but returns a small snapshot for logging / APIs.
    pub async fn file_stat(&self, path: impl AsRef<Path>) -> anyhow::Result<FileStat> {
        let meta = self.file_metadata(path).await?;
        Ok(FileStat::from_metadata(&meta))
    }
}

/// Snapshot of [`std::fs::Metadata`] for any filesystem path (file or directory).
#[derive(Debug, Clone)]
pub struct FileStat {
    pub len: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub readonly: bool,
}

impl FileStat {
    fn from_metadata(meta: &std::fs::Metadata) -> Self {
        Self {
            len: meta.len(),
            is_file: meta.is_file(),
            is_dir: meta.is_dir(),
            is_symlink: meta.is_symlink(),
            modified: meta.modified().ok(),
            accessed: meta.accessed().ok(),
            created: meta.created().ok(),
            readonly: meta.permissions().readonly(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    #[tokio::test]
    async fn read_and_file_stat_roundtrip() {
        let base = std::env::temp_dir().join(format!(
            "vnext_semantic_search_fs_test_{}",
            std::process::id()
        ));
        tokio::fs::create_dir_all(&base).await.expect("mkdir");
        let file_path = base.join("a.txt");
        tokio::fs::write(&file_path, b"hello").await.expect("write");

        let svc = FileService::new();
        let bytes = svc.read_file(&file_path).await.expect("read");
        assert_eq!(bytes, b"hello");
        let s = svc.read_file_to_string(&file_path).await.expect("read str");
        assert_eq!(s, "hello");

        let dir_stat = svc.file_stat(&base).await.expect("dir stat");
        assert!(dir_stat.is_dir);
        assert!(!dir_stat.is_file);
        assert!(dir_stat.modified.is_some());
        assert!(
            dir_stat
                .modified
                .unwrap()
                .duration_since(UNIX_EPOCH)
                .is_ok()
        );

        let file_stat = svc.file_stat(&file_path).await.expect("file stat");
        assert!(file_stat.is_file);
        assert!(!file_stat.is_dir);
        assert_eq!(file_stat.len, 5);

        let meta = svc.file_metadata(&base).await.expect("meta");
        assert!(meta.is_dir());

        let _ = tokio::fs::remove_dir_all(&base).await;
    }
}
