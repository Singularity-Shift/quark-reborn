use chrono::Utc;
use quark_core::helpers::dto::{CoinVersion, CreateProposalRequest};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ProposalStatus {
    Pending,
    Active,
    Completed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DaoAdminPreferences {
    pub group_id: String,
    pub expiration_time: u64,
    pub interval_active_proposal_notifications: u64,
    pub interval_dao_results_notifications: u64,
    pub default_dao_token: Option<String>,
    pub vote_duration: Option<u64>, // Duration in seconds for how long votes are open
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProposalEntry {
    pub name: String,
    pub description: String,
    pub options: Vec<String>,
    pub start_date: u64,
    pub end_date: u64,
    pub group_id: String,
    pub proposal_id: String,
    pub version: CoinVersion,
    pub coin_type: String,
    pub status: ProposalStatus,
    pub last_active_notification: u64,
    pub last_result_notification: u64,
    pub disabled_notifications: bool,
}

impl From<&CreateProposalRequest> for ProposalEntry {
    fn from(request: &CreateProposalRequest) -> Self {
        let now = Utc::now().timestamp() as u64;

        Self {
            name: request.name.clone(),
            description: request.description.clone(),
            options: request.options.clone(),
            start_date: request.start_date,
            end_date: request.end_date,
            group_id: request.group_id.clone(),
            proposal_id: request.proposal_id.clone(),
            version: request.version.clone(),
            coin_type: request.currency.clone(),
            status: ProposalStatus::Pending,
            last_active_notification: now,
            last_result_notification: now,
            disabled_notifications: false,
        }
    }
}
