use serde::{Deserialize, Serialize};
use teloxide::{
    dispatching::dialogue::{InMemStorage, Storage},
    types::ChatId,
};

/// One stored line.
#[derive(Clone, Serialize, Deserialize)]
pub struct MessageEntry {
    pub sender: Option<String>,
    pub text: String,
}

/// Per-chat buffer (max 20).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct MessageHistory(pub Vec<MessageEntry>);

/// Handy alias used everywhere else.
pub type HistoryStorage = std::sync::Arc<InMemStorage<MessageHistory>>;

/// Fetch the buffer (may be empty).
#[allow(dead_code)]
pub async fn fetch(chat_id: ChatId, storage: HistoryStorage) -> Vec<MessageEntry> {
    storage
        .get_dialogue(chat_id)
        .await
        .unwrap_or_default()
        .unwrap_or_default()
        .0
}
