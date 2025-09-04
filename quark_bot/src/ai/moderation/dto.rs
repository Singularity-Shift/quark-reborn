use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ModerationResult {
    pub verdict: String, // "P" or "F"
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModerationOverrides {
    pub allowed_items: Vec<String>,
    pub disallowed_items: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ModerationSettings {
    pub allowed_items: Vec<String>,
    pub disallowed_items: Vec<String>,
    pub updated_by_user_id: i64,
    pub updated_at_unix_ms: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ModerationState {
    pub step: String,
    pub allowed_items: Option<Vec<String>>,
    pub message_id: Option<i64>,
    #[serde(default)]
    pub started_by_user_id: Option<i64>,
}

impl From<(String, Option<Vec<String>>, Option<i64>)> for ModerationState {
    fn from(value: (String, Option<Vec<String>>, Option<i64>)) -> Self {
        let (step, allowed_items, message_id) = value;
        Self { step, allowed_items, message_id, started_by_user_id: None }
    }
}

impl From<(String, Option<Vec<String>>, Option<i64>, i64)> for ModerationState {
    fn from(value: (String, Option<Vec<String>>, Option<i64>, i64)) -> Self {
        let (step, allowed_items, message_id, started_by_user_id) = value;
        Self { step, allowed_items, message_id, started_by_user_id: Some(started_by_user_id) }
    }
}

impl From<(Vec<String>, Vec<String>, i64, i64)> for ModerationSettings {
    fn from(value: (Vec<String>, Vec<String>, i64, i64)) -> Self {
        let (allowed_items, disallowed_items, updated_by_user_id, updated_at_unix_ms) = value;

        Self {
            allowed_items,
            disallowed_items,
            updated_by_user_id,
            updated_at_unix_ms,
        }
    }
}
