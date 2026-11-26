use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct ApiClient {
    base_url: String,
    client: Client,
    token: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Implant {
    pub agent_id: String,
    pub hostname: String,
    pub username: String,
    pub status: String,
    pub sleep_ms: u64,
    pub last_checkin: String,
}

#[derive(Debug, Deserialize)]
pub struct TaskInfo {
    pub id: i64,
    pub task_type: String,
    pub status: String,
    pub output: Option<String>,
    pub success: Option<bool>,
}

#[derive(Serialize)]
struct LoginReq {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginResp {
    token: String,
}

#[derive(Serialize)]
struct QueueTaskReq {
    agent_id: String,
    task_type: u8,
    data: String,
}

#[derive(Deserialize)]
struct QueueTaskResp {
    task_id: i64,
}

#[derive(Deserialize)]
struct ImplantsResp {
    implants: Vec<Implant>,
}

#[derive(Deserialize)]
struct TasksResp {
    tasks: Vec<TaskInfo>,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::new(),
            token: None,
        }
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), String> {
        let resp: LoginResp = self
            .client
            .post(format!("{}/api/auth/login", self.base_url))
            .json(&LoginReq {
                username: username.into(),
                password: password.into(),
            })
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        self.token = Some(resp.token);
        Ok(())
    }

    pub async fn list_implants(&self) -> Result<Vec<Implant>, String> {
        let mut req = self.client.get(format!("{}/api/implants", self.base_url));
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let resp: ImplantsResp = req
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        Ok(resp.implants)
    }

    pub async fn queue_task(
        &self,
        agent_id: &str,
        task_type: u8,
        data: &str,
    ) -> Result<i64, String> {
        let mut req = self
            .client
            .post(format!("{}/api/tasks", self.base_url))
            .json(&QueueTaskReq {
                agent_id: agent_id.into(),
                task_type,
                data: data.into(),
            });
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let resp: QueueTaskResp = req
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        Ok(resp.task_id)
    }

    pub async fn list_tasks(&self, agent_id: &str) -> Result<Vec<TaskInfo>, String> {
        let mut req = self.client.get(format!(
            "{}/api/implants/{}/tasks",
            self.base_url, agent_id
        ));
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let resp: TasksResp = req
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        Ok(resp.tasks)
    }

    pub async fn get_task(&self, task_id: i64) -> Result<TaskInfo, String> {
        let mut req = self
            .client
            .get(format!("{}/api/tasks/{}", self.base_url, task_id));
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        req.send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }
}
