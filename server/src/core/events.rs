use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub event_type: String,
    pub agent_id: Option<String>,
    pub task_id: Option<i64>,
    pub detail: String,
    pub timestamp: String,
}

impl Event {
    pub fn new(event_type: &str, detail: &str) -> Self {
        Self {
            event_type: event_type.into(),
            agent_id: None,
            task_id: None,
            detail: detail.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_agent(mut self, agent_id: &str) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn with_task(mut self, task_id: i64) -> Self {
        self.task_id = Some(task_id);
        self
    }
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: Event) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}
