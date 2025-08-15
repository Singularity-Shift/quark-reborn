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


