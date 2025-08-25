use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SummarizerState {
    pub summary: Option<String>,
    pub last_rollover_unix: i64,
}
