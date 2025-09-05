//! Group file upload state management for quark_bot.

use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Simple state manager for tracking which groups are awaiting file uploads
#[derive(Clone)]
pub struct GroupFileUploadState {
    awaiting_groups: Arc<Mutex<HashSet<String>>>,
}

impl GroupFileUploadState {
    pub fn new() -> Self {
        Self {
            awaiting_groups: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn set_awaiting(&self, group_id: String) {
        let mut awaiting = self.awaiting_groups.lock().await;
        awaiting.insert(group_id);
    }

    pub async fn clear_awaiting(&self, group_id: String) {
        let mut awaiting = self.awaiting_groups.lock().await;
        awaiting.remove(&group_id);
    }

    pub async fn is_awaiting(&self, group_id: String) -> bool {
        let awaiting = self.awaiting_groups.lock().await;
        awaiting.contains(&group_id)
    }
}
