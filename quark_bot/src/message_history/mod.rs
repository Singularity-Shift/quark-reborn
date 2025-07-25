use serde::{Deserialize, Serialize};
use teloxide::{
    dispatching::dialogue::{InMemStorage, Storage},
    types::{ChatId, Message},
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

impl MessageHistory {
    pub fn push(&mut self, entry: MessageEntry) {
        if self.0.len() >= 20 {
            self.0.remove(0); // drop oldest
        }
        self.0.push(entry);
    }
}

/// Handy alias used everywhere else.
pub type HistoryStorage = std::sync::Arc<InMemStorage<MessageHistory>>;

/// Log a new group text.
pub async fn log(msg: &Message, storage: HistoryStorage) -> anyhow::Result<()> {
    if msg.chat.is_private() || msg.text().is_none() {
        return Ok(()); // skip DMs & non-text
    }

    let sender = msg
        .from
        .as_ref()
        .and_then(|u| u.username.clone().or_else(|| Some(u.first_name.clone())));

    // Fetch, mutate, save.
    let mut state = storage.clone().get_dialogue(msg.chat.id).await?.unwrap_or_default();

    let mut text = msg.text().unwrap().to_owned();
    const MAX_CHARS: usize = 500;
    if text.chars().count() > MAX_CHARS {
        text = text.chars().take(MAX_CHARS).collect();
        text.push('…');
    }

    state.push(MessageEntry {
        sender,
        text,
    });
    storage.clone().update_dialogue(msg.chat.id, state).await?;
    Ok(())
}

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