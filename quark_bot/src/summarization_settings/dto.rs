use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SummarizationPrefs {
    pub summarizer_enabled: Option<bool>,
    pub summarizer_token_limit: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct EffectiveSummarizationPrefs {
    pub enabled: bool,
    pub token_limit: u32,
}
