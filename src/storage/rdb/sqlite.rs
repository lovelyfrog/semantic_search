use std::{
    io,
    path::{Path, PathBuf},
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::types::Type;
use rusqlite::{Connection, Params, Result, params};

use crate::common::{
    data::{IndexStatus, IndexType, Project},
    utils::hash_str,
};

pub struct IndexStatusStore {
    db_path: PathBuf,
}

impl IndexStatusStore {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }
}

impl IndexStatusStore {
    pub fn open(&self) -> Result<Connection> {
        let conn = Connection::open(self.db_path.as_path())?;
        conn.execute_batch(include_str!("schema.sql"))?;
        Ok(conn)
    }

    fn get_project(&self, root_path: &Path, embedding_model: &str) -> Result<Option<Project>> {
        let root_canon = root_path
            .canonicalize()
            .map_err(|e| rusqlite::Error::InvalidPath(root_path.to_path_buf()))?;
        let root_str = root_canon.to_string_lossy();

        let conn = self.open()?;
        let mut stmt = conn.prepare(
            "SELECT id, root_path, embedding_model, hash, index_finished_time FROM projects 
            WHERE root_path = ?1 AND embedding_model = ?2",
        )?;

        let mut rows = stmt.query(params![root_str.as_ref(), embedding_model])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Project {
                id: row.get(0)?,
                root_path: PathBuf::from(row.get::<_, String>(1)?),
                embedding_model: row.get(2)?,
                hash: row.get(3)?,
                index_finished_time: Some(row.get(4)?),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_or_create_project(
        &self,
        root_path: &Path,
        embedding_model: &str,
    ) -> Result<Project> {
        if let Some(project) = self.get_project(root_path, embedding_model)? {
            return Ok(project);
        }

        let root_canon = root_path
            .canonicalize()
            .map_err(|e| rusqlite::Error::InvalidPath(root_path.to_path_buf()))?;
        let root_str = root_canon.to_string_lossy();

        let conn = self.open()?;

        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        let hash = hash_str(root_str.as_ref());
        let mut stmt = conn.prepare(
            "INSERT INTO projects (root_path, embedding_model, hash, index_finished_time) 
            VALUES (?1, ?2, ?3, ?4)",
        )?;
        stmt.execute(params![
            root_str.as_ref(),
            embedding_model,
            hash,
            created_at
        ])?;

        let project_id = conn.last_insert_rowid();
        let project = Project {
            id: project_id,
            root_path: root_path.to_path_buf(),
            embedding_model: embedding_model.to_string(),
            hash,
            index_finished_time: None,
        };
        Ok(project)
    }

    pub fn delete_project(&self, root_path: &Path, embedding_model: &str) -> Result<()> {
        let root_canon = root_path
            .canonicalize()
            .map_err(|e| rusqlite::Error::InvalidPath(root_path.to_path_buf()))?;
        let root_str = root_canon.to_string_lossy();

        let conn = self.open()?;
        let mut stmt =
            conn.prepare("DELETE FROM projects WHERE root_path = ?1 AND embedding_model = ?2")?;
        stmt.execute(params![root_str.as_ref(), embedding_model])?;
        Ok(())
    }

    pub fn delete_project_by_id(&self, project_id: i64) -> Result<()> {
        let conn = self.open()?;
        let mut stmt = conn.prepare("DELETE FROM projects WHERE id = ?1")?;
        stmt.execute(params![project_id])?;
        Ok(())
    }

    fn delete_all_projects(&self) -> Result<()> {
        let conn = self.open()?;
        let mut stmt = conn.prepare("DELETE FROM projects")?;
        stmt.execute(params![])?;
        Ok(())
    }

    pub fn update_project(&self, project_id: i64, index_finished_time: u64) -> Result<()> {
        let conn = self.open()?;
        let mut stmt =
            conn.prepare("UPDATE projects SET index_finished_time = ?1 WHERE id = ?2")?;
        stmt.execute(params![index_finished_time, project_id])?;
        Ok(())
    }

    pub fn get_project_index_finished_time(&self, project_id: i64) -> Result<Option<u64>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare("SELECT index_finished_time FROM projects WHERE id = ?1")?;
        let mut rows = stmt.query(params![project_id])?;
        if let Some(row) = rows.next()? {
            Ok(row.get(0)?)
        } else {
            Ok(None)
        }
    }

    pub fn get_index_status_by_project(
        &self,
        project_id: i64,
        layer: IndexType,
    ) -> Result<Vec<IndexStatus>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            "SELECT project_id, file_path, layer, file_hash, mtime, ctime, size, indexed_at 
            FROM index_status 
            WHERE project_id = ?1 AND layer = ?2",
        )?;
        let mut rows = stmt.query_map(params![project_id, layer.to_string()], |row| {
            Ok(IndexStatus {
                project_id: row.get(0)?,
                file_path: row.get(1)?,
                layer: IndexType::from_str(row.get::<_, String>(2)?.as_str()).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        Type::Text,
                        Box::new(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
                    )
                })?,
                file_hash: row.get(3)?,
                mtime: row.get(4)?,
                ctime: row.get(5)?,
                size: row.get(6)?,
                indexed_at: row.get(7)?,
            })
        })?;

        let mut index_statuses = Vec::new();
        for row in rows {
            index_statuses.push(row?);
        }
        Ok(index_statuses)
    }

    pub fn get_index_status_by_path(
        &self,
        project_id: i64,
        file_path: &str,
        layer: IndexType,
    ) -> Result<Option<IndexStatus>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            "SELECT project_id, file_path, layer, file_hash, mtime, ctime, size, indexed_at 
            FROM index_status 
            WHERE project_id = ?1 
                AND file_path = ?2 
                AND layer = ?3",
        )?;
        let mut rows = stmt.query(params![project_id, file_path, layer.to_string()])?;
        if let Some(row) = rows.next()? {
            Ok(Some(IndexStatus {
                project_id: row.get(0)?,
                file_path: row.get(1)?,
                layer: IndexType::from_str(row.get::<_, String>(2)?.as_str()).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        Type::Text,
                        Box::new(io::Error::new(io::ErrorKind::InvalidData, e.to_string())),
                    )
                })?,
                file_hash: row.get(3)?,
                mtime: row.get(4)?,
                ctime: row.get(5)?,
                size: row.get(6)?,
                indexed_at: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn upsert_index_status(&self, index_status: &IndexStatus) -> Result<()> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            "INSERT INTO index_status (project_id, file_path, layer, file_hash, mtime, ctime, size, indexed_at) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) 
            ON CONFLICT (project_id, file_path, layer) 
            DO UPDATE SET 
                file_hash = excluded.file_hash, 
                mtime = excluded.mtime, 
                ctime = excluded.ctime, 
                size = excluded.size, 
                indexed_at = excluded.indexed_at"
        )?;
        stmt.execute(params![
            index_status.project_id,
            index_status.file_path,
            index_status.layer.to_string(),
            index_status.file_hash,
            index_status.mtime,
            index_status.ctime,
            index_status.size,
            index_status.indexed_at
        ])?;
        Ok(())
    }

    pub fn delete_index_status_by_project(&self, project_id: i64, layer: IndexType) -> Result<()> {
        let conn = self.open()?;
        let mut stmt =
            conn.prepare("DELETE FROM index_status WHERE project_id = ?1 AND layer = ?2")?;
        stmt.execute(params![project_id, layer.to_string()])?;
        Ok(())
    }

    pub fn delete_index_status_by_path(
        &self,
        project_id: i64,
        file_path: &str,
        layer: IndexType,
    ) -> Result<()> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            "DELETE FROM index_status WHERE project_id = ?1 AND file_path = ?2 AND layer = ?3",
        )?;
        stmt.execute(params![project_id, file_path, layer.to_string()])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::utils::temp_dir;

    use super::*;

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs()
    }

    fn setup_store() -> IndexStatusStore {
        let db_path = temp_dir().join("db/test.db");
        IndexStatusStore::new(db_path)
    }
    #[test]
    fn test_create_project() {
        let store = setup_store();
        let project_path1 = temp_dir().join("projects/hmosworld/commons/aspect");
        let project_path2 = temp_dir().join("projects/hmosworld/commons/audioplayer");
        let project1 = store.get_or_create_project(&project_path1, "veso").unwrap();
        let project2 = store.get_or_create_project(&project_path2, "veso").unwrap();
        assert_eq!(project1.id, 1);
        assert_eq!(project2.id, 2);
        assert_eq!(project1.root_path, project_path1);
        assert_eq!(project2.root_path, project_path2);
        assert_eq!(project1.embedding_model, "veso");
        assert_eq!(project2.embedding_model, "veso");
    }

    #[test]
    fn test_delete_project_by_id() {
        let store = setup_store();
        let project_path = temp_dir().join("projects/hmosworld/commons/aspect");
        let project = store.get_or_create_project(&project_path, "veso").unwrap();
        assert_eq!(project.id, 1);
        store.delete_project_by_id(project.id).unwrap();
        let project = store.get_project(&project_path, "veso").unwrap();
        assert!(project.is_none());
    }

    #[test]
    fn test_delete_project_by_path() {
        let store = setup_store();
        let project_path = temp_dir().join("projects/hmosworld/commons/aspect");
        let project = store.get_or_create_project(&project_path, "veso").unwrap();
        assert_eq!(project.id, 1);
        store.delete_project(&project_path, "veso").unwrap();
        let project = store.get_project(&project_path, "veso").unwrap();
        assert!(project.is_none());
    }

    #[test]
    fn test_delete_all_projects() {
        let store = setup_store();
        let project_path1 = temp_dir().join("projects/hmosworld/commons/aspect");
        let project_path2 = temp_dir().join("projects/hmosworld/commons/audioplayer");
        let project1 = store.get_or_create_project(&project_path1, "veso").unwrap();
        let project2 = store.get_or_create_project(&project_path2, "veso").unwrap();
        store.delete_all_projects().unwrap();
        let project1 = store.get_project(&project_path1, "veso").unwrap();
        assert!(project1.is_none());
        let project2 = store.get_project(&project_path2, "veso").unwrap();
        assert!(project2.is_none());
    }

    #[test]
    fn test_insert_and_delete_project_index_status() {
        let store = setup_store();
        let project_path = temp_dir().join("projects/hmosworld/commons/aspect");
        let project = store.get_or_create_project(&project_path, "veso").unwrap();
        let index_status = IndexStatus {
            project_id: project.id,
            file_path: "test.txt".to_string(),
            layer: IndexType::File,
            file_hash: "test".to_string(),
            mtime: now(),
            ctime: now(),
            size: 100,
            indexed_at: now(),
        };
        store.upsert_index_status(&index_status).unwrap();
        let index_status = store
            .get_index_status_by_project(project.id, IndexType::File)
            .unwrap();
        assert_eq!(index_status.len(), 1);
        assert_eq!(index_status[0].file_path, "test.txt");
        assert_eq!(index_status[0].layer, IndexType::File);
        assert_eq!(index_status[0].file_hash, "test");
        assert_eq!(index_status[0].mtime, now());
        assert_eq!(index_status[0].ctime, now());

        store.delete_project_by_id(project.id).unwrap();
        let index_status = store
            .get_index_status_by_project(project.id, IndexType::File)
            .unwrap();
        assert!(index_status.is_empty());
    }

    #[test]
    fn test_update_project() {
        let store = setup_store();
        let project_path = temp_dir().join("projects/hmosworld/commons/aspect");
        let project = store.get_or_create_project(&project_path, "veso").unwrap();
        assert_eq!(project.index_finished_time, None);
        store.update_project(project.id, 64).unwrap();
        let project = store.get_project(&project_path, "veso").unwrap();
        assert!(project.is_some());
        assert_eq!(project.unwrap().index_finished_time, Some(64));
    }

    #[test]
    fn test_delete_file() {
        let store = setup_store();
        let project_path = temp_dir().join("projects/hmosworld/commons/aspect");
        let project = store.get_or_create_project(&project_path, "veso").unwrap();
        let index_status = IndexStatus {
            project_id: project.id,
            file_path: "test.txt".to_string(),
            layer: IndexType::File,
            file_hash: "test".to_string(),
            mtime: now(),
            ctime: now(),
            size: 100,
            indexed_at: now(),
        };
        store.upsert_index_status(&index_status).unwrap();
        store
            .delete_index_status_by_path(project.id, &index_status.file_path, IndexType::File)
            .unwrap();
        let index_status = store
            .get_index_status_by_path(project.id, &index_status.file_path, IndexType::File)
            .unwrap();
        assert!(index_status.is_none());
    }

    #[test]
    fn test_upsert_update() {
        let store = setup_store();
        let project_path = temp_dir().join("projects/hmosworld/commons/aspect");
        let project = store.get_or_create_project(&project_path, "veso").unwrap();
        let index_status = IndexStatus {
            project_id: project.id,
            file_path: "test.txt".to_string(),
            layer: IndexType::File,
            file_hash: "test".to_string(),
            mtime: now(),
            ctime: now(),
            size: 100,
            indexed_at: now(),
        };
        store.upsert_index_status(&index_status).unwrap();
        let index_status = store
            .get_index_status_by_path(project.id, &index_status.file_path, IndexType::File)
            .unwrap();
        assert!(index_status.is_some());
        assert_eq!(index_status.unwrap().file_hash, "test");
        let index_status = IndexStatus {
            project_id: project.id,
            file_path: "test.txt".to_string(),
            layer: IndexType::File,
            file_hash: "test2".to_string(),
            mtime: now(),
            ctime: now(),
            size: 100,
            indexed_at: now(),
        };
        store.upsert_index_status(&index_status).unwrap();
        let index_status = store
            .get_index_status_by_path(project.id, &index_status.file_path, IndexType::File)
            .unwrap();
        assert!(index_status.is_some());
        assert_eq!(index_status.unwrap().file_hash, "test2");
    }
}
