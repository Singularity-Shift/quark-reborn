use anyhow::Result;
use quark_core::helpers::jwt::JwtManager;
use serde_json;
use sled::Tree;
use teloxide::types::UserId;

use crate::credentials::dto::Credentials;

pub fn get_credentials(username: &str, db: Tree) -> Option<Credentials> {
    let bytes_op = db.get(username).unwrap();

    if let Some(bytes) = bytes_op {
        let credentials: Credentials = serde_json::from_slice(&bytes).unwrap();
        Some(credentials)
    } else {
        None
    }
}

pub fn save_credentials(username: &str, credentials: Credentials, db: Tree) -> Result<()> {
    let bytes = serde_json::to_vec(&credentials).unwrap();
    db.insert(username, bytes).map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

pub async fn generate_new_jwt(
    username: String,
    user_id: UserId,
    account_address: String,
    jwt_manager: JwtManager,
    db: Tree,
) -> bool {
    match jwt_manager.generate_token(user_id, account_address.clone()) {
        Ok(token) => {
            let jwt = token;

            let credentials = Credentials::from((jwt, user_id, account_address));

            let saved = save_credentials(&username, credentials, db);

            if saved.is_err() {
                println!("❌ Failed to save credentials: {}", saved.err().unwrap());
                return false;
            }

            println!("✅ Generated new JWT token for user {}", user_id);
            return true;
        }
        Err(e) => {
            println!("❌ Failed to generate JWT token: {}", e);
            return false;
        }
    }
}
