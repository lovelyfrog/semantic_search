use std::sync::Arc;

use tokio::sync::mpsc;

use crate::{
    common::data::{Chunk, ChunkMsg, IndexType, Project},
    embedding::{onnx::OnnxEmbedder, utils::Embedder},
    index::utils::ProgressReporter,
    metrics::{
        profiler::IndexProfiler,
        timer::{Stage, StageTimer},
    },
    storage::manager::StorageManager,
};

pub struct IndexWorker {
    embedder: Arc<OnnxEmbedder>,
    storage_manager: Arc<StorageManager>,
    num_threads: usize,
    embed_batch_size: usize,
}

impl IndexWorker {
    pub fn new(
        embedder: Arc<OnnxEmbedder>,
        storage_manager: Arc<StorageManager>,
        num_threads: usize,
        embed_batch_size: usize,
    ) -> Self {
        Self {
            embedder,
            storage_manager,
            num_threads,
            embed_batch_size,
        }
    }

    pub async fn embed(
        &self,
        mut chunk_rx: mpsc::Receiver<ChunkMsg>,
        embed_tx: mpsc::Sender<ChunkMsg>,
        layer: IndexType,
        profiler: Arc<IndexProfiler>,
    ) {
        let all_batches = self.embed_batch_size * self.num_threads;
        let mut buffer = Vec::with_capacity(all_batches);

        while let Some(msg) = chunk_rx.recv().await {
            match msg {
                ChunkMsg::Chunk(chunk) => {
                    buffer.push(chunk);
                    if buffer.len() == all_batches {
                        self.process_batch_embed(&mut buffer, &embed_tx, layer, profiler.clone())
                            .await;
                        buffer.clear();
                    }
                }
                _ => {
                    embed_tx
                        .send(msg)
                        .await
                        .unwrap_or_else(|e| log::error!("failed to send chunk for store: {}", e));
                }
            }
        }

        if !buffer.is_empty() {
            self.process_batch_embed(&mut buffer, &embed_tx, layer, profiler.clone())
                .await;
            buffer.clear();
        }
    }

    async fn process_batch_embed(
        &self,
        buffer: &mut Vec<Chunk>,
        embed_tx: &mpsc::Sender<ChunkMsg>,
        layer: IndexType,
        profiler: Arc<IndexProfiler>,
    ) {
        let inputs: Vec<String> = buffer
            .iter()
            .map(|chunk| chunk.embedding_content.clone())
            .collect();

        log::info!("embedding {} chunks into layer {}", inputs.len(), layer);

        let mut handles = Vec::new();

        let timer = StageTimer::new(profiler.metrics(), Stage::Embedding, layer);
        for chunk in inputs.chunks(self.embed_batch_size) {
            let embedder = self.embedder.clone();
            let data = chunk.to_vec();

            let handle = std::thread::spawn(move || {
                let embeddings = embedder.batch_embed(&data);
                embeddings
            });
            handles.push(handle);
        }

        let mut batch_embeddings = Vec::with_capacity(inputs.len());
        for handle in handles {
            match handle.join() {
                Ok(Ok(embeddings)) => {
                    batch_embeddings.extend(embeddings);
                }
                Ok(Err(e)) => {
                    log::error!("failed to embed chunks: {}", e);
                }
                Err(_) => {
                    log::error!("failed to join embed thread");
                }
            }
        }

        timer.finish();

        log::info!(
            "embedded {} chunks into layer {}",
            batch_embeddings.len(),
            layer
        );
        for (chunk, embedding) in buffer.iter_mut().zip(batch_embeddings.iter()) {
            chunk.embedding = embedding.clone();
            embed_tx
                .send(ChunkMsg::Chunk(chunk.clone()))
                .await
                .unwrap_or_else(|e| log::error!("failed to send chunk for store: {}", e));
        }
    }

    pub async fn insert(
        &self,
        project: Project,
        layer: IndexType,
        mut embed_rx: mpsc::Receiver<ChunkMsg>,
        profiler: Arc<IndexProfiler>,
        progress_reporter: Arc<dyn ProgressReporter>,
    ) {
        const BATCH_SIZE: usize = 128;
        let mut buffer = Vec::with_capacity(BATCH_SIZE);

        while let Some(msg) = embed_rx.recv().await {
            match msg {
                ChunkMsg::Chunk(chunk) => {
                    buffer.push(chunk);
                    if buffer.len() == BATCH_SIZE {
                        self.process_batch_insert(
                            project.clone(),
                            layer,
                            &buffer,
                            profiler.clone(),
                        )
                        .await;
                        buffer.clear();
                    }
                }
                ChunkMsg::FileStart { file_path } => {
                    log::info!("deleting file {} into layer {}", file_path, layer);
                    self.storage_manager
                        .delete_chunks(&file_path, layer)
                        .await
                        .unwrap_or_else(|e| {
                            log::error!("failed to delete chunks for file {}: {}", file_path, e)
                        });
                }
                ChunkMsg::FileEnd { file_status } => {
                    log::info!(
                        "inserting file status {} into layer {}",
                        file_status.file_path,
                        layer
                    );
                    self.storage_manager
                        .upsert_index_status(&file_status)
                        .unwrap_or_else(|e| {
                            log::error!(
                                "failed to upsert index status for file {}: {}",
                                file_status.file_path,
                                e
                            )
                        });

                    profiler.inc_file(layer);
                    progress_reporter.on_progress(profiler.index_progress());
                }
            }
        }

        if !buffer.is_empty() {
            self.process_batch_insert(project, layer, &buffer, profiler.clone())
                .await;
            buffer.clear();
        }

        profiler.finish_layer(layer);
        if profiler.is_finished() {
            let metrics = profiler.stop_profiler();

            let index_finished_time = metrics.end_time.as_secs();
            let _ = self
                .storage_manager
                .update_project_index_finished_time(index_finished_time);
            progress_reporter.on_completed();
        }
    }

    async fn process_batch_insert(
        &self,
        _project: Project,
        layer: IndexType,
        buffer: &Vec<Chunk>,
        profiler: Arc<IndexProfiler>,
    ) {
        log::info!("inserting {} chunks into layer {}", buffer.len(), layer);

        log::debug!("chunks: {:?}", buffer);

        let timer = StageTimer::new(profiler.metrics(), Stage::DbWrite, layer);
        self.storage_manager
            .append_chunks(layer, buffer.clone())
            .await
            .unwrap_or_else(|e| log::error!("failed to append chunks: {}", e));
        timer.finish();

        log::info!("inserted {} chunks into layer {}", buffer.len(), layer);
    }
}
