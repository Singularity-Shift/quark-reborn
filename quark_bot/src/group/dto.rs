use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupCredentials {
    pub jwt: String,
    pub group_id: String,
    pub resource_account_address: String,
    pub users: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupCredentialsPayload {
    #[serde(rename = "resourceAccountAddress")]
    pub resource_account_address: String,
}

impl From<(String, String, String, Vec<String>)> for GroupCredentials {
    fn from(value: (String, String, String, Vec<String>)) -> Self {
        let (jwt, group_id, resource_account_address, users) = value;

        GroupCredentials {
            jwt,
            group_id,
            resource_account_address,
            users,
        }
    }
}
