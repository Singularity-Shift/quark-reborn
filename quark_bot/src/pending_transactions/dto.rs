use quark_core::helpers::dto::CoinVersion;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PendingTransaction {
    pub transaction_id: String,         // Unique UUID for this transaction
    pub amount: u64,                    // Amount in smallest units
    pub users: Vec<String>,             // User addresses  
    pub coin_type: String,              // Token address/type
    pub version: CoinVersion,           // V1 or V2
    pub jwt_token: String,              // JWT for authentication
    pub is_group_transfer: bool,        // Whether this is a group or individual transfer
    pub symbol: String,                 // Token symbol for display
    pub user_addresses: Vec<String>,    // Recipient addresses
    pub original_usernames: Vec<String>, // Original usernames for display
    pub per_user_amount: f64,           // Amount per user (for display)
    pub created_at: u64,                // Timestamp when created
    pub expires_at: u64,                // Timestamp when transaction expires
    pub chat_id: i64,                   // Telegram chat ID where the message was sent
    pub message_id: i32,                // Telegram message ID of the transaction message
}