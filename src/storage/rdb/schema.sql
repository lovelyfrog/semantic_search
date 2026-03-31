PRAGMA foreign_keys = ON;

-- projects: project metadata
CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    root_path TEXT NOT NULL,
    embedding_model TEXT NOT NULL,
    hash TEXT NOT NULL,
    index_finished_time INTEGER NOT NULL,
    UNIQUE (root_path, embedding_model)
);

-- index_status: index status of each file
CREATE TABLE IF NOT EXISTS index_status (
    project_id INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    layer TEXT NOT NULL, -- file, symbol, content
    file_hash TEXT NOT NULL,
    mtime INTEGER NOT NULL,
    ctime INTEGER NOT NULL,
    size INTEGER NOT NULL,
    indexed_at INTEGER NOT NULL,
    PRIMARY KEY (project_id, file_path, layer),
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS idx_project_layer 
    ON index_status (project_id, layer);