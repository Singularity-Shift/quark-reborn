use serde::{Deserialize, Serialize};
use teloxide::types::UserId;

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub jwt: String,
    pub user_id: UserId,
    pub account_address: String,
}

impl From<(String, UserId, String)> for Credentials {
    fn from(value: (String, UserId, String)) -> Self {
        let (jwt, user_id, account_address) = value;

        Credentials {
            jwt,
            user_id,
            account_address,
        }
    }
}
