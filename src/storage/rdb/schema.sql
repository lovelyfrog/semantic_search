PRAGMA foreign_keys = ON;

-- 单库单行工程元数据：主键为 (root_path, embedding_model)，不使用自增 id。
CREATE TABLE IF NOT EXISTS projects (
    root_path TEXT NOT NULL,
    embedding_model TEXT NOT NULL,
    hash TEXT NOT NULL,
    index_finished_time INTEGER NOT NULL,
    PRIMARY KEY (root_path, embedding_model)
);

-- File index state; no project_id (database is already scoped to one project).
CREATE TABLE IF NOT EXISTS index_status (
    file_path TEXT NOT NULL,
    layer TEXT NOT NULL, -- file, symbol, content
    file_hash TEXT NOT NULL,
    mtime INTEGER NOT NULL,
    ctime INTEGER NOT NULL,
    size INTEGER NOT NULL,
    indexed_at INTEGER NOT NULL,
    PRIMARY KEY (file_path, layer)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS idx_index_status_layer 
    ON index_status (layer);
