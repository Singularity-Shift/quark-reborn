use anyhow::Result;
use serde_json;
use sled::{Db, Tree};
use std::time::{SystemTime, UNIX_EPOCH};

use super::auth::{current_timestamp, is_state_expired};
use super::dto::{OAuthState, TwitterUserV2};

/// Storage helper for Twitter OAuth operations
pub struct TwitterStorage {
    db: Db,
}

impl TwitterStorage {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Get the OAuth states tree
    pub fn oauth_states_tree(&self) -> Result<Tree> {
        Ok(self.db.open_tree("oauth_states")?)
    }

    /// Get the Twitter auth tree
    pub fn twitter_auth_tree(&self) -> Result<Tree> {
        Ok(self.db.open_tree("twitter_auth_v2")?)
    }

    /// Store OAuth state with TTL
    pub fn store_oauth_state(&self, state: &str, oauth_state: &OAuthState) -> Result<()> {
        let tree = self.oauth_states_tree()?;
        let state_json = serde_json::to_vec(oauth_state)?;
        tree.insert(state, state_json)?;
        Ok(())
    }

    /// Retrieve and validate OAuth state
    pub fn get_oauth_state(&self, state: &str) -> Result<Option<OAuthState>> {
        let tree = self.oauth_states_tree()?;
        
        if let Some(state_bytes) = tree.get(state)? {
            let oauth_state: OAuthState = serde_json::from_slice(&state_bytes)?;
            
            // Check if state has expired
            if is_state_expired(oauth_state.created_at) {
                // Remove expired state
                tree.remove(state)?;
                return Ok(None);
            }
            
            Ok(Some(oauth_state))
        } else {
            Ok(None)
        }
    }

    /// Remove OAuth state (after successful authentication)
    pub fn remove_oauth_state(&self, state: &str) -> Result<()> {
        let tree = self.oauth_states_tree()?;
        tree.remove(state)?;
        Ok(())
    }

    /// Store Twitter user data
    pub fn store_twitter_user(&self, user: &TwitterUserV2) -> Result<()> {
        let tree = self.twitter_auth_tree()?;
        let user_json = serde_json::to_vec(user)?;
        
        // Store by Twitter handle (primary key)
        tree.insert(&user.twitter_handle, user_json.clone())?;
        
        // Also store by Telegram username for lookups
        let telegram_key = format!("tg:{}", user.telegram_username);
        tree.insert(telegram_key, user_json.clone())?;
        
        // Store by Twitter ID for ID-based lookups
        let twitter_id_key = format!("id:{}", user.twitter_id);
        tree.insert(twitter_id_key, user_json)?;
        
        Ok(())
    }

    /// Get Twitter user by handle
    pub fn get_twitter_user_by_handle(&self, handle: &str) -> Result<Option<TwitterUserV2>> {
        let tree = self.twitter_auth_tree()?;
        
        if let Some(user_bytes) = tree.get(handle)? {
            let user: TwitterUserV2 = serde_json::from_slice(&user_bytes)?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// Get Twitter user by Telegram username
    pub fn get_twitter_user_by_telegram(&self, telegram_username: &str) -> Result<Option<TwitterUserV2>> {
        let tree = self.twitter_auth_tree()?;
        let key = format!("tg:{}", telegram_username);
        
        if let Some(user_bytes) = tree.get(key)? {
            let user: TwitterUserV2 = serde_json::from_slice(&user_bytes)?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// Get Twitter user by Twitter ID
    pub fn get_twitter_user_by_id(&self, twitter_id: u64) -> Result<Option<TwitterUserV2>> {
        let tree = self.twitter_auth_tree()?;
        let key = format!("id:{}", twitter_id);
        
        if let Some(user_bytes) = tree.get(key)? {
            let user: TwitterUserV2 = serde_json::from_slice(&user_bytes)?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    /// Update Twitter user data (for refreshing profile info)
    pub fn update_twitter_user(&self, updated_user: &TwitterUserV2) -> Result<()> {
        // Remove old entries if handle changed
        if let Ok(Some(existing_user)) = self.get_twitter_user_by_telegram(&updated_user.telegram_username) {
            if existing_user.twitter_handle != updated_user.twitter_handle {
                self.remove_twitter_user(&existing_user)?;
            }
        }
        
        // Store updated user
        self.store_twitter_user(updated_user)
    }

    /// Remove Twitter user (all key variants)
    pub fn remove_twitter_user(&self, user: &TwitterUserV2) -> Result<()> {
        let tree = self.twitter_auth_tree()?;
        
        // Remove by handle
        tree.remove(&user.twitter_handle)?;
        
        // Remove by Telegram username
        let telegram_key = format!("tg:{}", user.telegram_username);
        tree.remove(telegram_key)?;
        
        // Remove by Twitter ID
        let twitter_id_key = format!("id:{}", user.twitter_id);
        tree.remove(twitter_id_key)?;
        
        Ok(())
    }

    /// List all qualified Twitter users
    pub fn get_qualified_users(&self) -> Result<Vec<TwitterUserV2>> {
        let tree = self.twitter_auth_tree()?;
        let mut qualified_users = Vec::new();
        
        for result in tree.iter() {
            let (key, value) = result?;
            let key_str = String::from_utf8_lossy(&key);
            
            // Only process entries that are Twitter handles (not prefixed with tg: or id:)
            if !key_str.starts_with("tg:") && !key_str.starts_with("id:") {
                let user: TwitterUserV2 = serde_json::from_slice(&value)?;
                if user.qualifies {
                    qualified_users.push(user);
                }
            }
        }
        
        Ok(qualified_users)
    }

    /// Clean up expired OAuth states
    pub fn cleanup_expired_oauth_states(&self) -> Result<usize> {
        let tree = self.oauth_states_tree()?;
        let mut removed_count = 0;
        let current_time = current_timestamp();
        
        let mut states_to_remove = Vec::new();
        
        for result in tree.iter() {
            let (key, value) = result?;
            if let Ok(oauth_state) = serde_json::from_slice::<OAuthState>(&value) {
                if is_state_expired(oauth_state.created_at) {
                    states_to_remove.push(key.to_vec());
                }
            }
        }
        
        for key in states_to_remove {
            tree.remove(key)?;
            removed_count += 1;
        }
        
        Ok(removed_count)
    }

    /// Get user count statistics
    pub fn get_user_stats(&self) -> Result<(usize, usize)> {
        let tree = self.twitter_auth_tree()?;
        let mut total_users = 0;
        let mut qualified_users = 0;
        
        for result in tree.iter() {
            let (key, value) = result?;
            let key_str = String::from_utf8_lossy(&key);
            
            // Only count entries that are Twitter handles (not prefixed with tg: or id:)
            if !key_str.starts_with("tg:") && !key_str.starts_with("id:") {
                total_users += 1;
                
                let user: TwitterUserV2 = serde_json::from_slice(&value)?;
                if user.qualifies {
                    qualified_users += 1;
                }
            }
        }
        
        Ok((total_users, qualified_users))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twitter::auth;
    use tempfile::TempDir;

    fn create_test_storage() -> (TwitterStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db = sled::open(temp_dir.path()).unwrap();
        let storage = TwitterStorage::new(db);
        (storage, temp_dir)
    }

    fn create_test_user() -> TwitterUserV2 {
        TwitterUserV2 {
            telegram_username: "testuser".to_string(),
            telegram_user_id: 12345,
            twitter_handle: "testhandle".to_string(),
            twitter_id: 67890,
            access_token: "encrypted_token".to_string(),
            refresh_token: Some("encrypted_refresh".to_string()),
            scopes: vec!["tweet.read".to_string(), "users.read".to_string()],
            follower_count: 100,
            has_profile_pic: true,
            has_banner_pic: true,
            verified: false,
            qualifies: true,
            checked_at: auth::current_timestamp(),
            version: 2,
        }
    }

    #[test]
    fn test_oauth_state_storage() {
        let (storage, _temp) = create_test_storage();
        
        let oauth_state = OAuthState {
            telegram_user_id: 12345,
            telegram_username: "testuser".to_string(),
            verifier: "test_verifier".to_string(),
            nonce: "test_nonce".to_string(),
            created_at: auth::current_timestamp(),
        };
        
        let state_key = "test_state";
        
        // Store state
        storage.store_oauth_state(state_key, &oauth_state).unwrap();
        
        // Retrieve state
        let retrieved = storage.get_oauth_state(state_key).unwrap();
        assert!(retrieved.is_some());
        
        let retrieved_state = retrieved.unwrap();
        assert_eq!(retrieved_state.telegram_user_id, oauth_state.telegram_user_id);
        assert_eq!(retrieved_state.verifier, oauth_state.verifier);
    }

    #[test]
    fn test_twitter_user_storage() {
        let (storage, _temp) = create_test_storage();
        let user = create_test_user();
        
        // Store user
        storage.store_twitter_user(&user).unwrap();
        
        // Retrieve by handle
        let by_handle = storage.get_twitter_user_by_handle(&user.twitter_handle).unwrap();
        assert!(by_handle.is_some());
        assert_eq!(by_handle.unwrap().twitter_handle, user.twitter_handle);
        
        // Retrieve by Telegram username
        let by_telegram = storage.get_twitter_user_by_telegram(&user.telegram_username).unwrap();
        assert!(by_telegram.is_some());
        assert_eq!(by_telegram.unwrap().telegram_username, user.telegram_username);
        
        // Retrieve by Twitter ID
        let by_id = storage.get_twitter_user_by_id(user.twitter_id).unwrap();
        assert!(by_id.is_some());
        assert_eq!(by_id.unwrap().twitter_id, user.twitter_id);
    }
} 