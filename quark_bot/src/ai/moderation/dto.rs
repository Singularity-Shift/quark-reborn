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

// Implementations intentionally omitted to keep dto.rs data-only
