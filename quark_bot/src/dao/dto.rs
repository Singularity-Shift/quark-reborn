use chrono::Utc;
use quark_core::helpers::dto::{CoinVersion, CreateDaoRequest};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum DaoStatus {
    Pending,
    Active,
    Completed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DaoAdminPreferences {
    pub group_id: String,
    pub expiration_time: u64,
    pub interval_active_dao_notifications: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DaoEntry {
    pub name: String,
    pub description: String,
    pub options: Vec<String>,
    pub start_date: u64,
    pub end_date: u64,
    pub group_id: String,
    pub dao_id: String,
    pub version: CoinVersion,
    pub coin_type: String,
    pub status: DaoStatus,
    pub last_active_notification: u64,
    pub result_notified: bool,
}

impl From<&CreateDaoRequest> for DaoEntry {
    fn from(request: &CreateDaoRequest) -> Self {
        let now = Utc::now().timestamp() as u64;

        Self {
            name: request.name.clone(),
            description: request.description.clone(),
            options: request.options.clone(),
            start_date: request.start_date,
            end_date: request.end_date,
            group_id: request.group_id.clone(),
            dao_id: request.dao_id.clone(),
            version: request.version.clone(),
            coin_type: request.currency.clone(),
            status: DaoStatus::Pending,
            last_active_notification: now,
            result_notified: false,
        }
    }
}
