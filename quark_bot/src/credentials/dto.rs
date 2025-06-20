use serde::{Deserialize, Serialize};
use teloxide::types::UserId;

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub jwt: String,
    pub user_id: UserId,
    pub account_address: String,
    pub resource_account_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialsPayload {
    #[serde(rename = "accountAddress")]
    pub account_address: String,
    #[serde(rename = "resourceAccountAddress")]
    pub resource_account_address: String,
}

impl From<(String, UserId, String, String)> for Credentials {
    fn from(value: (String, UserId, String, String)) -> Self {
        let (jwt, user_id, account_address, resource_account_address) = value;

        Credentials {
            jwt,
            user_id,
            account_address,
            resource_account_address,
        }
    }
}
