use crate::core::implants::ImplantRegistry;
use crate::core::taskqueue::TaskQueue;
use crate::storage::db::Database;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub implants: Arc<ImplantRegistry>,
    pub tasks: Arc<TaskQueue>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let db = Arc::new(db);
        let implants = Arc::new(ImplantRegistry::new(db.clone()));
        let tasks = Arc::new(TaskQueue::new(db.clone()));
        Self {
            db,
            implants,
            tasks,
        }
    }
}
