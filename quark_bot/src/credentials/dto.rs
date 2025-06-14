use serde::{Deserialize, Serialize};
use teloxide::types::UserId;

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub jwt: String,
    pub user_id: UserId,
}

impl From<(String, UserId)> for Credentials {
    fn from(value: (String, UserId)) -> Self {
        let (jwt, user_id) = value;

        Credentials { jwt, user_id }
    }
}
