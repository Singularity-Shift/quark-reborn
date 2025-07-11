
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
    pub tools_used: Vec<ToolUsage>,
    pub group_id: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct PurchaseMessage {
    #[schema(value_type = String)]
    pub model: Model,
    pub tokens_used: u32,
    pub tools_used: Vec<ToolUsage>,
    pub account_address: String,
    pub group_id: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct ToolUsage {
    pub tool: AITool,
    pub calls: u32,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub enum AITool {
    FileSearch,
    ImageGeneration,
    WebSearchPreview,
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
pub struct TransactionResponse {
    pub hash: String,
}

#[derive(Deserialize, Debug)]
pub struct SimulateTransactionResponse {
    pub success: bool,
    pub vm_status: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenAddress {
    pub vec: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceCoin {
    pub chain_id: Option<u64>,
    pub panora_id: Option<String>,
    pub token_address: Option<String>,
    pub fa_address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub usd_price: Option<String>,
    pub native_price: Option<String>,
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
        let group_id = request.group_id;

        PurchaseMessage {
            model,
            tokens_used,
            tools_used,
            account_address,
            group_id,
        }
    }
}
