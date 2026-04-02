use std::{fmt, time::Duration};

use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
pub struct IndexMetrics {
    pub total_time: Duration,

    pub embedding_file_time: Duration,
    pub db_write_file_time: Duration,

    pub embedding_symbol_time: Duration,
    pub db_write_symbol_time: Duration,

    pub start_memory: u64,
    pub peak_memory: u64,
    pub avg_memory: u64,

    pub start_cpu: f32,
    pub peak_cpu: f32,
    pub avg_cpu: f32,

    pub start_storage: u64,
    pub end_storage: u64,

    pub handled_file_count: usize,
    pub handled_symbol_count: usize,

    pub total_file_count: usize,
    pub total_symbol_count: usize,

    pub start_time: Duration,
    pub end_time: Duration,
}

impl fmt::Display for IndexMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "IndexMetrics {{")?;
        writeln!(f, "   total_time: {:.2}s", self.total_time.as_secs_f64())?;
        writeln!(
            f,
            "   embedding_file_time: {:.2}s",
            self.embedding_file_time.as_secs_f64()
        )?;
        writeln!(
            f,
            "   db_write_file_time: {:.2}s",
            self.db_write_file_time.as_secs_f64()
        )?;
        writeln!(
            f,
            "   embedding_symbol_time: {:.2}s",
            self.embedding_symbol_time.as_secs_f64()
        )?;
        writeln!(
            f,
            "   db_write_symbol_time: {:.2}s",
            self.db_write_symbol_time.as_secs_f64()
        )?;
        writeln!(
            f,
            "   start_memory: {:.2}MB",
            self.start_memory as f64 / 1024.0 / 1024.0
        )?;
        writeln!(
            f,
            "   peak_memory: {:.2}MB",
            self.peak_memory as f64 / 1024.0 / 1024.0
        )?;
        writeln!(
            f,
            "   avg_memory: {:.2}MB",
            self.avg_memory as f64 / 1024.0 / 1024.0
        )?;
        writeln!(f, "   start_cpu: {:.2}%", self.start_cpu)?;
        writeln!(f, "   peak_cpu: {:.2}%", self.peak_cpu)?;
        writeln!(f, "   avg_cpu: {:.2}%", self.avg_cpu)?;
        writeln!(
            f,
            "   start_storage: {:.2}MB",
            self.start_storage as f64 / 1024.0 / 1024.0
        )?;
        writeln!(
            f,
            "   end_storage: {:.2}MB",
            self.end_storage as f64 / 1024.0 / 1024.0
        )?;
        writeln!(f, "   total_file_count: {}", self.total_file_count)?;
        writeln!(f, "   total_symbol_count: {}", self.total_symbol_count)?;
        writeln!(f, "   start_time: {:.2}s", self.start_time.as_secs_f64())?;
        writeln!(f, "   end_time: {:.2}s", self.end_time.as_secs_f64())?;
        writeln!(f, "}}")
    }
}
