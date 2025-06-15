use quark_core::helpers::jwt::JwtManager;
use sled::Tree;
use teloxide::types::Message;

use crate::credentials::helpers::{generate_new_jwt, get_credentials};

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

<<<<<<< HEAD
    println!("❌ No credentials found for user {}", username);
    return false;
=======
    return generate_new_jwt(username, user.id, jwt_manager, db).await;
}

async fn generate_new_jwt(
    username: String,
    user_id: UserId,
    jwt_manager: JwtManager,
    db: Tree,
) -> bool {
    match jwt_manager.generate_token(user_id) {
        Ok(token) => {
            let jwt = token;

            let credentials = Credentials::from((jwt, user_id));

            let saved = save_credentials(&username, credentials, db);

            if saved.is_err() {
                println!("❌ Failed to save credentials: {}", saved.err().unwrap());
                return false;
            }

            return true;
        }
        Err(e) => {
            println!("❌ Failed to generate JWT token: {}", e);
            return false;
        }
    }
>>>>>>> main
}
