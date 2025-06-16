use serde::{Deserialize, Serialize};
use teloxide::types::UserId;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub enum PayUsersVersion {
    V1,
    V2,
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

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct PayUsersRequest {
    pub amount: u64,
    pub users: Vec<String>,
    pub coin_type: String,
    pub version: PayUsersVersion,
}
