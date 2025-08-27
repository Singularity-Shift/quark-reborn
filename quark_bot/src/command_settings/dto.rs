use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandSettings {
    pub group_id: String,
    pub chat_commands_enabled: bool,
}

impl Default for CommandSettings {
    fn default() -> Self {
        Self {
            group_id: String::new(),
            chat_commands_enabled: true, // Default to enabled
        }
    }
}

impl From<String> for CommandSettings {
    fn from(group_id: String) -> Self {
        Self {
            group_id,
            chat_commands_enabled: true,
        }
    }
}
