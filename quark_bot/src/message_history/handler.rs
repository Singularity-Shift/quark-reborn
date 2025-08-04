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

/// Store a new message entry in the rolling buffer (max 20 messages).
pub async fn store_message(
    chat_id: ChatId,
    entry: MessageEntry,
    storage: HistoryStorage,
) {
    // Clone storage so we can use it twice
    let storage_clone = storage.clone();
    
    let current_history = storage
        .get_dialogue(chat_id)
        .await
        .unwrap_or_default()
        .unwrap_or_default();
        
    let mut messages = current_history.0;
    messages.push(entry);
    
    // Keep only the most recent 20 entries.
    if messages.len() > 20 {
        let excess = messages.len() - 20;
        messages.drain(0..excess);
    }
    
    let new_history = MessageHistory(messages);
    storage_clone
        .update_dialogue(chat_id, new_history)
        .await
        .expect("Failed to update message history");
}
