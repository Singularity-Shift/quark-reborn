use crate::credentials::helpers::{generate_new_jwt, get_credentials};
use quark_core::helpers::jwt::JwtManager;
use sled::Tree;
use teloxide::types::Message;

pub async fn auth(msg: Message, db: Tree) -> bool {
    let jwt_manager = JwtManager::new();

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

    let credentials_opt = get_credentials(&username, db.clone());

    if let Some(credentials) = credentials_opt {
        // Initialize JWT manager and validate/update storage
        match jwt_manager.validate_and_update_jwt(
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

        return generate_new_jwt(
            username,
            user.id,
            credentials.account_address,
            jwt_manager,
            db,
        )
        .await;
    }

    println!("❌ No credentials found for user {}", username);
    return false;
}
