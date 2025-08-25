use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SummarizerState {
    pub summary: Option<String>,
    pub last_rollover_unix: i64,
    pub pending_thread_clear: bool,
}

#[derive(Debug, Clone)]
pub struct SummarizationResult {
    pub summary: String,
    pub total_tokens: u32,
}
