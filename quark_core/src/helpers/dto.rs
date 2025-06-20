use serde::{Deserialize, Serialize};
use std::{env, fmt};
use teloxide::types::UserId;
use utoipa::ToSchema;

pub enum Endpoints {
    PayUsers,
    Purchase,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub telegram_id: UserId,
    pub exp: i64, // Expiration time
    pub iat: i64, // Issued at
    pub account_address: String,
}

#[derive(Debug, Clone)]
pub struct UserPayload {
    pub account_address: String,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct PurchaseRequest {
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub enum PayUsersVersion {
    V1,
    V2,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct PayUsersRequest {
    pub amount: u64,
    pub users: Vec<String>,
    pub coin_type: String,
    pub version: PayUsersVersion,
}

impl fmt::Display for Endpoints {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let backend_host =
            env::var("BACKEND_URL").expect("BACKEND_URL environment variable not set");
        let backend_url = backend_host;

        match self {
            &Endpoints::PayUsers => write!(f, "{}/pay-users", backend_url),
            &Endpoints::Purchase => write!(f, "{}/purchase", backend_url),
        }
    }
}
