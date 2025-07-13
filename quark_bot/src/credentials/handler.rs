use anyhow::Result;
use quark_core::helpers::jwt::JwtManager;
use serde_json;
use sled::Tree;
use teloxide::types::{Message, UserId};

use crate::credentials::dto::Credentials;

#[derive(Clone)]
pub struct Auth {
    jwt_manager: JwtManager,
    db: Tree,
}

impl Auth {
    pub fn new(db: Tree) -> Self {
        let jwt_manager = JwtManager::new();

        Self { jwt_manager, db }
    }

    pub fn get_credentials(&self, username: &str) -> Option<Credentials> {
        let bytes_op = self.db.get(username).unwrap();

        if let Some(bytes) = bytes_op {
            let credentials: Credentials = serde_json::from_slice(&bytes).unwrap();
            Some(credentials)
        } else {
            None
        }
    }

    pub fn save_credentials(&self, username: &str, credentials: Credentials) -> Result<()> {
        let bytes = serde_json::to_vec(&credentials).unwrap();
        self.db
            .insert(username, bytes)
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(())
    }

    pub async fn generate_new_jwt(
        &self,
        username: String,
        user_id: UserId,
        account_address: String,
        resource_account_address: String,
    ) -> bool {
        let account_address = account_address.clone();

        match self
            .jwt_manager
            .generate_token(user_id, account_address.clone())
        {
            Ok(token) => {
                let jwt = token;

                let credentials =
                    Credentials::from((jwt, user_id, account_address, resource_account_address));

                let saved = self.save_credentials(&username, credentials);

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

    pub async fn verify(&self, msg: Message) -> bool {
        let user = msg.from;

        if user.is_none() {
            return false;
        }

        let user = user.unwrap();

        let username = user.username;

        if username.is_none() {
            return false;
        }

        let username = username.unwrap();

        let credentials_opt = self.get_credentials(&username);

        if let Some(credentials) = credentials_opt {
            // Initialize JWT manager and validate/update storage
            match self.jwt_manager.validate_and_update_jwt(
                credentials.jwt,
                credentials.user_id,
                credentials.account_address.clone(),
            ) {
                Ok(_updated_storage) => {
                    // Note: The updated storage with the new JWT would need to be
                    // persisted back to the dialogue storage in the calling code
                    return true;
                }
                Err(e) => {
                    log::warn!("AUTH: Failed to validate/generate JWT: {}", e);
                }
            }

            return self
                .generate_new_jwt(
                    username,
                    user.id,
                    credentials.account_address,
                    credentials.resource_account_address,
                )
                .await;
        }

        println!("❌ No credentials found for user {}", username);
        return false;
    }
}
