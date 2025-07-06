use open_ai_rust_responses_by_sshift::Model;
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
    #[schema(value_type = String)]
    pub model: Model,
    pub tokens_used: u32,
    pub tools_used: AITool,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct PurchaseMessage {
    #[schema(value_type = String)]
    pub model: Model,
    pub tokens_used: u32,
    pub tools_used: AITool,
    pub account_address: String,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub enum AITool {
    FileSearch,
    GPTImage1,
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

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct PayUsersResponse {
    pub hash: String,
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

impl From<(PurchaseRequest, String)> for PurchaseMessage {
    fn from((request, account_address): (PurchaseRequest, String)) -> Self {
        let model = request.model;
        let tokens_used = request.tokens_used;
        let tools_used = request.tools_used;

        PurchaseMessage {
            model,
            tokens_used,
            tools_used,
            account_address,
        }
    }
}
