use rusqlite::{params, Connection, Result as SqlResult};
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    pub fn open_in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS implants (
                agent_id    BLOB(16) PRIMARY KEY,
                hostname    TEXT NOT NULL,
                os_version  TEXT NOT NULL DEFAULT '',
                username    TEXT NOT NULL DEFAULT '',
                internal_ip TEXT NOT NULL DEFAULT '',
                external_ip TEXT NOT NULL DEFAULT '',
                first_seen  TEXT NOT NULL,
                last_checkin TEXT NOT NULL,
                sleep_ms    INTEGER NOT NULL DEFAULT 30000,
                jitter_pct  INTEGER NOT NULL DEFAULT 20,
                listener_id TEXT NOT NULL DEFAULT ''
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_id      BLOB(16) NOT NULL REFERENCES implants(agent_id),
                task_type     INTEGER NOT NULL,
                data          BLOB NOT NULL DEFAULT x'',
                status        TEXT NOT NULL DEFAULT 'pending',
                created_at    TEXT NOT NULL,
                dispatched_at TEXT,
                completed_at  TEXT,
                output        BLOB,
                success       INTEGER
            );

            CREATE TABLE IF NOT EXISTS events (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type  TEXT NOT NULL,
                agent_id    BLOB(16),
                task_id     INTEGER,
                detail      TEXT,
                created_at  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS loot (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_id    BLOB(16) NOT NULL REFERENCES implants(agent_id),
                loot_type   TEXT NOT NULL,
                filename    TEXT NOT NULL,
                disk_path   TEXT NOT NULL,
                size_bytes  INTEGER NOT NULL DEFAULT 0,
                created_at  TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_tasks_agent_status
                ON tasks(agent_id, status);
            CREATE INDEX IF NOT EXISTS idx_events_created
                ON events(created_at);
            CREATE INDEX IF NOT EXISTS idx_loot_agent
                ON loot(agent_id);",
        )?;
        Ok(())
    }

    // --- Implants ---

    pub fn upsert_implant(
        &self,
        agent_id: &[u8; 16],
        hostname: &str,
        sleep_ms: u32,
        external_ip: &str,
        listener_id: &str,
    ) -> SqlResult<bool> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let existing: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM implants WHERE agent_id = ?1",
                params![agent_id.as_slice()],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)?;

        if existing {
            conn.execute(
                "UPDATE implants SET hostname = ?1, last_checkin = ?2, sleep_ms = ?3,
                 external_ip = ?4, listener_id = ?5 WHERE agent_id = ?6",
                params![hostname, now, sleep_ms, external_ip, listener_id, agent_id.as_slice()],
            )?;
        } else {
            conn.execute(
                "INSERT INTO implants (agent_id, hostname, first_seen, last_checkin, sleep_ms,
                 external_ip, listener_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    agent_id.as_slice(),
                    hostname,
                    now,
                    now,
                    sleep_ms,
                    external_ip,
                    listener_id
                ],
            )?;
        }
        Ok(!existing)
    }

    pub fn list_implants(&self) -> SqlResult<Vec<ImplantRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT agent_id, hostname, os_version, username, internal_ip, external_ip,
                    first_seen, last_checkin, sleep_ms, jitter_pct, listener_id
             FROM implants ORDER BY last_checkin DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                let id_blob: Vec<u8> = row.get(0)?;
                let mut agent_id = [0u8; 16];
                if id_blob.len() == 16 {
                    agent_id.copy_from_slice(&id_blob);
                }
                Ok(ImplantRow {
                    agent_id,
                    hostname: row.get(1)?,
                    os_version: row.get(2)?,
                    username: row.get(3)?,
                    internal_ip: row.get(4)?,
                    external_ip: row.get(5)?,
                    first_seen: row.get(6)?,
                    last_checkin: row.get(7)?,
                    sleep_ms: row.get(8)?,
                    jitter_pct: row.get(9)?,
                    listener_id: row.get(10)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get_implant(&self, agent_id: &[u8; 16]) -> SqlResult<Option<ImplantRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT agent_id, hostname, os_version, username, internal_ip, external_ip,
                    first_seen, last_checkin, sleep_ms, jitter_pct, listener_id
             FROM implants WHERE agent_id = ?1",
        )?;
        let mut rows = stmt.query_map(params![agent_id.as_slice()], |row| {
            let id_blob: Vec<u8> = row.get(0)?;
            let mut agent_id = [0u8; 16];
            if id_blob.len() == 16 {
                agent_id.copy_from_slice(&id_blob);
            }
            Ok(ImplantRow {
                agent_id,
                hostname: row.get(1)?,
                os_version: row.get(2)?,
                username: row.get(3)?,
                internal_ip: row.get(4)?,
                external_ip: row.get(5)?,
                first_seen: row.get(6)?,
                last_checkin: row.get(7)?,
                sleep_ms: row.get(8)?,
                jitter_pct: row.get(9)?,
                listener_id: row.get(10)?,
            })
        })?;
        rows.next().transpose()
    }

    pub fn delete_implant(&self, agent_id: &[u8; 16]) -> SqlResult<bool> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM tasks WHERE agent_id = ?1",
            params![agent_id.as_slice()],
        )?;
        let changed = conn.execute(
            "DELETE FROM implants WHERE agent_id = ?1",
            params![agent_id.as_slice()],
        )?;
        Ok(changed > 0)
    }

    // --- Tasks ---

    pub fn queue_task(
        &self,
        agent_id: &[u8; 16],
        task_type: u8,
        data: &[u8],
    ) -> SqlResult<i64> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO tasks (agent_id, task_type, data, status, created_at)
             VALUES (?1, ?2, ?3, 'pending', ?4)",
            params![agent_id.as_slice(), task_type, data, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn drain_pending_tasks(&self, agent_id: &[u8; 16]) -> SqlResult<Vec<TaskRow>> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT id, task_type, data FROM tasks
             WHERE agent_id = ?1 AND status = 'pending'
             ORDER BY id ASC",
        )?;
        let rows = stmt
            .query_map(params![agent_id.as_slice()], |row| {
                Ok(TaskRow {
                    id: row.get(0)?,
                    task_type: row.get(1)?,
                    data: row.get(2)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        if !rows.is_empty() {
            let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
            let placeholders: String = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!(
                "UPDATE tasks SET status = 'dispatched', dispatched_at = ?1 WHERE id IN ({})",
                placeholders
            );
            let mut stmt = conn.prepare(&sql)?;
            let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> =
                vec![Box::new(now)];
            for id in &ids {
                params_vec.push(Box::new(*id));
            }
            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();
            stmt.execute(param_refs.as_slice())?;
        }
        Ok(rows)
    }

    pub fn complete_task(
        &self,
        task_id: i64,
        success: bool,
        output: &[u8],
    ) -> SqlResult<bool> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let status = if success { "completed" } else { "failed" };
        let changed = conn.execute(
            "UPDATE tasks SET status = ?1, completed_at = ?2, success = ?3, output = ?4
             WHERE id = ?5",
            params![status, now, success as i32, output, task_id],
        )?;
        Ok(changed > 0)
    }

    pub fn get_task(&self, task_id: i64) -> SqlResult<Option<FullTaskRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, task_type, data, status, created_at, dispatched_at,
                    completed_at, output, success
             FROM tasks WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![task_id], |row| {
            let id_blob: Vec<u8> = row.get(1)?;
            let mut agent_id = [0u8; 16];
            if id_blob.len() == 16 {
                agent_id.copy_from_slice(&id_blob);
            }
            Ok(FullTaskRow {
                id: row.get(0)?,
                agent_id,
                task_type: row.get(2)?,
                data: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                dispatched_at: row.get(6)?,
                completed_at: row.get(7)?,
                output: row.get(8)?,
                success: row.get(9)?,
            })
        })?;
        rows.next().transpose()
    }

    pub fn list_tasks_for_implant(&self, agent_id: &[u8; 16]) -> SqlResult<Vec<FullTaskRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, task_type, data, status, created_at, dispatched_at,
                    completed_at, output, success
             FROM tasks WHERE agent_id = ?1 ORDER BY id DESC",
        )?;
        let rows = stmt
            .query_map(params![agent_id.as_slice()], |row| {
                let id_blob: Vec<u8> = row.get(1)?;
                let mut agent_id = [0u8; 16];
                if id_blob.len() == 16 {
                    agent_id.copy_from_slice(&id_blob);
                }
                Ok(FullTaskRow {
                    id: row.get(0)?,
                    agent_id,
                    task_type: row.get(2)?,
                    data: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                    dispatched_at: row.get(6)?,
                    completed_at: row.get(7)?,
                    output: row.get(8)?,
                    success: row.get(9)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(rows)
    }

    // --- Events ---

    // --- Loot ---

    pub fn insert_loot(
        &self,
        agent_id: &[u8; 16],
        loot_type: &str,
        filename: &str,
        disk_path: &str,
        size_bytes: i64,
    ) -> SqlResult<i64> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO loot (agent_id, loot_type, filename, disk_path, size_bytes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![agent_id.as_slice(), loot_type, filename, disk_path, size_bytes, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_loot(&self) -> SqlResult<Vec<LootRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, loot_type, filename, disk_path, size_bytes, created_at
             FROM loot ORDER BY id DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                let id_blob: Vec<u8> = row.get(1)?;
                let mut agent_id = [0u8; 16];
                if id_blob.len() == 16 {
                    agent_id.copy_from_slice(&id_blob);
                }
                Ok(LootRow {
                    id: row.get(0)?,
                    agent_id,
                    loot_type: row.get(2)?,
                    filename: row.get(3)?,
                    disk_path: row.get(4)?,
                    size_bytes: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn list_loot_for_implant(&self, agent_id: &[u8; 16]) -> SqlResult<Vec<LootRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, loot_type, filename, disk_path, size_bytes, created_at
             FROM loot WHERE agent_id = ?1 ORDER BY id DESC",
        )?;
        let rows = stmt
            .query_map(params![agent_id.as_slice()], |row| {
                let id_blob: Vec<u8> = row.get(1)?;
                let mut agent_id = [0u8; 16];
                if id_blob.len() == 16 {
                    agent_id.copy_from_slice(&id_blob);
                }
                Ok(LootRow {
                    id: row.get(0)?,
                    agent_id,
                    loot_type: row.get(2)?,
                    filename: row.get(3)?,
                    disk_path: row.get(4)?,
                    size_bytes: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get_loot(&self, loot_id: i64) -> SqlResult<Option<LootRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, loot_type, filename, disk_path, size_bytes, created_at
             FROM loot WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![loot_id], |row| {
            let id_blob: Vec<u8> = row.get(1)?;
            let mut agent_id = [0u8; 16];
            if id_blob.len() == 16 {
                agent_id.copy_from_slice(&id_blob);
            }
            Ok(LootRow {
                id: row.get(0)?,
                agent_id,
                loot_type: row.get(2)?,
                filename: row.get(3)?,
                disk_path: row.get(4)?,
                size_bytes: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.next().transpose()
    }

    pub fn delete_loot(&self, loot_id: i64) -> SqlResult<bool> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute("DELETE FROM loot WHERE id = ?1", params![loot_id])?;
        Ok(changed > 0)
    }

    // --- Events ---

    pub fn log_event(
        &self,
        event_type: &str,
        agent_id: Option<&[u8; 16]>,
        task_id: Option<i64>,
        detail: &str,
    ) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO events (event_type, agent_id, task_id, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                event_type,
                agent_id.map(|a| a.as_slice()),
                task_id,
                detail,
                now
            ],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ImplantRow {
    pub agent_id: [u8; 16],
    pub hostname: String,
    pub os_version: String,
    pub username: String,
    pub internal_ip: String,
    pub external_ip: String,
    pub first_seen: String,
    pub last_checkin: String,
    pub sleep_ms: i64,
    pub jitter_pct: i64,
    pub listener_id: String,
}

#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: i64,
    pub task_type: u8,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct LootRow {
    pub id: i64,
    pub agent_id: [u8; 16],
    pub loot_type: String,
    pub filename: String,
    pub disk_path: String,
    pub size_bytes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct FullTaskRow {
    pub id: i64,
    pub agent_id: [u8; 16],
    pub task_type: u8,
    pub data: Vec<u8>,
    pub status: String,
    pub created_at: String,
    pub dispatched_at: Option<String>,
    pub completed_at: Option<String>,
    pub output: Option<Vec<u8>>,
    pub success: Option<i32>,
}
