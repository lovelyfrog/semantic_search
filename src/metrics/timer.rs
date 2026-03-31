use std::{sync::Arc, time::Instant};

use parking_lot::Mutex;

use crate::{common::data::IndexType, metrics::data::IndexMetrics};

pub struct StageTimer {
    start: Instant,
    metrics: Arc<Mutex<IndexMetrics>>,
    stage: Stage,
    layer: IndexType,
}

pub enum Stage {
    Embedding,
    DbWrite,
}

impl StageTimer {
    pub fn new(metrics: Arc<Mutex<IndexMetrics>>, stage: Stage, layer: IndexType) -> Self {
        Self {
            start: Instant::now(),
            metrics,
            stage,
            layer,
        }
    }

    pub fn finish(&self) {
        let elapsed = self.start.elapsed();
        let mut metrics = self.metrics.lock();
        match self.stage {
            Stage::Embedding => match self.layer {
                IndexType::File => metrics.embedding_file_time += elapsed,
                IndexType::Symbol => metrics.embedding_symbol_time += elapsed,
                _ => {}
            },
            Stage::DbWrite => match self.layer {
                IndexType::File => metrics.db_write_file_time += elapsed,
                IndexType::Symbol => metrics.db_write_symbol_time += elapsed,
                _ => {}
            },
        }
    }
}
