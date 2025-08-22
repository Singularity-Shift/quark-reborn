use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SponsorInterval {
    Hourly,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SponsorSettings {
    pub requests: u64,
    pub interval: SponsorInterval,
}

impl Default for SponsorSettings {
    fn default() -> Self {
        Self {
            requests: 0,
            interval: SponsorInterval::Hourly,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SponsorRequest {
    pub requests_left: u64,
    pub last_request: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SponsorStep {
    AwaitingRequestLimit,
    AwaitingInterval,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SponsorState {
    pub group_id: String,
    pub step: SponsorStep,
    pub message_id: Option<u32>,
    pub admin_user_id: Option<u64>,
}
