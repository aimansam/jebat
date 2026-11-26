use crate::storage::db::Database;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct FileStore {
    db: Arc<Database>,
    base_dir: PathBuf,
}

impl FileStore {
    pub fn new(db: Arc<Database>, base_dir: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(base_dir)?;
        Ok(Self {
            db,
            base_dir: base_dir.to_path_buf(),
        })
    }

    fn agent_dir(&self, agent_id: &[u8; 16]) -> PathBuf {
        self.base_dir.join(hex::encode(agent_id))
    }

    pub fn store_file(
        &self,
        agent_id: &[u8; 16],
        loot_type: &str,
        filename: &str,
        data: &[u8],
    ) -> Result<i64, String> {
        let dir = self.agent_dir(agent_id).join(loot_type);
        std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {}", e))?;

        let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let safe_name = sanitize_filename(filename);
        let disk_name = format!("{}_{}", ts, safe_name);
        let disk_path = dir.join(&disk_name);

        std::fs::write(&disk_path, data).map_err(|e| format!("write: {}", e))?;

        let disk_path_str = disk_path.to_string_lossy().to_string();
        self.db
            .insert_loot(agent_id, loot_type, filename, &disk_path_str, data.len() as i64)
            .map_err(|e| format!("db: {}", e))
    }

    pub fn read_file(&self, loot_id: i64) -> Result<(String, Vec<u8>), String> {
        let loot = self
            .db
            .get_loot(loot_id)
            .map_err(|e| format!("db: {}", e))?
            .ok_or_else(|| "not found".to_string())?;
        let data =
            std::fs::read(&loot.disk_path).map_err(|e| format!("read: {}", e))?;
        Ok((loot.filename, data))
    }

    pub fn delete_file(&self, loot_id: i64) -> Result<bool, String> {
        let loot = self
            .db
            .get_loot(loot_id)
            .map_err(|e| format!("db: {}", e))?;
        if let Some(l) = loot {
            let _ = std::fs::remove_file(&l.disk_path);
            self.db
                .delete_loot(loot_id)
                .map_err(|e| format!("db: {}", e))
        } else {
            Ok(false)
        }
    }

    pub fn store_task_output(
        &self,
        agent_id: &[u8; 16],
        task_type: u8,
        output: &[u8],
    ) -> Result<i64, String> {
        let loot_type = match task_type {
            6 => "credentials",
            4 | 5 => "files",
            _ => return Err("not a loot task type".into()),
        };
        let filename = match task_type {
            6 => "cred_dump.txt",
            _ => "file_data.bin",
        };
        self.store_file(agent_id, loot_type, filename, output)
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect()
}
