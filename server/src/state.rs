use crate::core::events::EventBus;
use crate::core::filestore::FileStore;
use crate::core::implants::ImplantRegistry;
use crate::core::taskqueue::TaskQueue;
use crate::storage::db::Database;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub implants: Arc<ImplantRegistry>,
    pub tasks: Arc<TaskQueue>,
    pub events: Arc<EventBus>,
    pub files: Arc<FileStore>,
}

impl AppState {
    pub fn new(db: Database, loot_dir: &Path) -> Self {
        let db = Arc::new(db);
        let implants = Arc::new(ImplantRegistry::new(db.clone()));
        let tasks = Arc::new(TaskQueue::new(db.clone()));
        let events = Arc::new(EventBus::new(256));
        let files = Arc::new(
            FileStore::new(db.clone(), loot_dir).expect("failed to create loot directory"),
        );
        Self {
            db,
            implants,
            tasks,
            events,
            files,
        }
    }
}
