use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use parking_lot::Mutex;
use sysinfo::{Pid, System};

use crate::{
    common::{data::IndexType, utils::construct_walker},
    index::utils::IndexProgress,
    metrics::data::IndexMetrics,
    storage::manager::StorageOptions,
};

pub struct IndexRunningStatus {
    file: Arc<AtomicBool>,
    symbol: Arc<AtomicBool>,
}

impl IndexRunningStatus {
    pub fn new() -> Self {
        Self {
            file: Arc::new(AtomicBool::new(true)),
            symbol: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn finish_file(&self) {
        self.file.store(false, Ordering::Relaxed);
    }

    pub fn finish_symbol(&self) {
        self.symbol.store(false, Ordering::Relaxed);
    }

    pub fn is_file_finished(&self) -> bool {
        !self.file.load(Ordering::Relaxed)
    }

    pub fn is_symbol_finished(&self) -> bool {
        !self.symbol.load(Ordering::Relaxed)
    }

    pub fn is_finished(&self) -> bool {
        self.is_file_finished() && self.is_symbol_finished()
    }
}

pub struct IndexProfiler {
    metrics: Arc<Mutex<IndexMetrics>>,
    running_status: Arc<IndexRunningStatus>,
    start_time: Instant,
    storage_options: StorageOptions,
}

impl IndexProfiler {
    pub fn new(storage_options: StorageOptions) -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let pid = sysinfo::get_current_pid().unwrap();
        let process = system.process(pid).unwrap();

        let metrics = IndexMetrics {
            total_time: Duration::default(),
            embedding_file_time: Duration::default(),
            db_write_file_time: Duration::default(),
            embedding_symbol_time: Duration::default(),
            db_write_symbol_time: Duration::default(),
            start_memory: process.memory(),
            peak_memory: process.memory(),
            avg_memory: process.memory(),
            start_cpu: process.cpu_usage(),
            peak_cpu: process.cpu_usage(),
            avg_cpu: process.cpu_usage(),
            start_storage: storage_size(&storage_options),
            end_storage: 0,
            handled_file_count: 0,
            handled_symbol_count: 0,
            total_file_count: 0,
            total_symbol_count: 0,
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default(),
            end_time: Duration::default(),
        };

        let metrics = Arc::new(Mutex::new(metrics));
        let running_status = Arc::new(IndexRunningStatus::new());

        Self::spawn_sampler(metrics.clone(), running_status.clone(), pid);

        Self {
            metrics,
            running_status,
            start_time: Instant::now(),
            storage_options,
        }
    }

    fn spawn_sampler(
        metrics: Arc<Mutex<IndexMetrics>>,
        running_status: Arc<IndexRunningStatus>,
        pid: Pid,
    ) {
        thread::spawn(move || {
            let mut system = System::new_all();

            let mut cpu_sum: f64 = 0.0;
            let mut memory_sum: u64 = 0;
            let mut samples: u64 = 0;

            while !running_status.is_finished() {
                system.refresh_process(pid);

                if let Some(process) = system.process(pid) {
                    let mut metrics = metrics.lock();

                    let memory = process.memory();
                    let cpu = process.cpu_usage();

                    metrics.peak_memory = metrics.peak_memory.max(memory);
                    metrics.peak_cpu = metrics.peak_cpu.max(cpu);

                    cpu_sum += cpu as f64;
                    memory_sum += memory;
                    samples += 1;

                    metrics.avg_cpu = (cpu_sum / samples as f64) as f32;
                    metrics.avg_memory = (memory_sum / samples) as u64;
                }

                thread::sleep(Duration::from_millis(500));
            }
        });
    }

    pub fn finish_layer(&self, layer: IndexType) {
        match layer {
            IndexType::File => self.running_status.finish_file(),
            IndexType::Symbol => self.running_status.finish_symbol(),
            _ => {}
        }
    }

    pub fn stop_profiler(&self) -> IndexMetrics {
        let mut metrics = self.metrics.lock();
        metrics.total_time = self.start_time.elapsed();
        metrics.end_storage = storage_size(&self.storage_options);
        metrics.end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        metrics.clone()
    }

    pub fn is_finished(&self) -> bool {
        self.running_status.is_finished()
    }

    pub fn metrics(&self) -> Arc<Mutex<IndexMetrics>> {
        self.metrics.clone()
    }

    pub fn inc_file(&self, layer: IndexType) {
        let mut metrics = self.metrics.lock();
        match layer {
            IndexType::File => metrics.handled_file_count += 1,
            IndexType::Symbol => metrics.handled_symbol_count += 1,
            _ => {}
        }
    }

    pub fn index_progress(&self) -> IndexProgress {
        let metrics = self.metrics.lock();
        IndexProgress {
            total_file_count: metrics.total_file_count,
            total_symbol_count: metrics.total_symbol_count,
            handled_file_count: metrics.handled_file_count,
            handled_symbol_count: metrics.handled_symbol_count,
        }
    }

    pub fn set_total_count(&self, layer: IndexType, total_count: usize) {
        let mut metrics = self.metrics.lock();
        match layer {
            IndexType::File => metrics.total_file_count = total_count,
            IndexType::Symbol => metrics.total_symbol_count = total_count,
            _ => {}
        }
    }
}

fn storage_size(storage_options: &StorageOptions) -> u64 {
    dir_size(&storage_options.vector_db_path) + file_size(&storage_options.index_db_path)
}

fn dir_size(path: &Path) -> u64 {
    let walker = construct_walker(path, true, &[], &[], None);
    walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.metadata().ok())
        .map(|metadata| metadata.len())
        .sum::<u64>()
}

fn file_size(path: &Path) -> u64 {
    if path.is_file() {
        path.metadata().map(|metadata| metadata.len()).unwrap_or(0)
    } else {
        0
    }
}
