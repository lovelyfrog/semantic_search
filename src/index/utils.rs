use std::sync::atomic::{AtomicBool, Ordering};

pub struct IndexProgress {
    pub total_file_count: usize,
    pub total_symbol_count: usize,
    pub handled_file_count: usize,
    pub handled_symbol_count: usize,
}

pub trait ProgressReporter: Send + Sync {
    fn on_progress(&self, progress: IndexProgress);
    fn on_completed(&self);
    fn on_error(&self, error: String);
}

pub struct ConsoleProgressReporter;

impl ConsoleProgressReporter {
    pub fn new() -> Self {
        Self {}
    }
}

impl ProgressReporter for ConsoleProgressReporter {
    fn on_progress(&self, progress: IndexProgress) {
        log::info!(
            "indexing...\n handled {} files and {} symbols\n total {} files and {} symbols",
            progress.handled_file_count,
            progress.handled_symbol_count,
            progress.total_file_count,
            progress.total_symbol_count,
        );
    }

    fn on_completed(&self) {
        log::info!("indexing completed");
    }

    fn on_error(&self, error: String) {
        log::error!("indexing failed: {}", error);
    }
}

pub trait CancelToken: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

pub struct SimpleCancelToken {
    cancelled: AtomicBool,
}

impl SimpleCancelToken {
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl CancelToken for SimpleCancelToken {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}
