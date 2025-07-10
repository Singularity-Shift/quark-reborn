use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TwitterUserV2 {
    pub telegram_username: String,      // Telegram username (without @)
    pub telegram_user_id: u64,          // Telegram user ID
    pub twitter_handle: String,         // Twitter username
    pub twitter_id: u64,                // Twitter numeric ID
    pub access_token: String,           // Will be encrypted before storage
    pub refresh_token: Option<String>,  // Optional refresh token
    pub scopes: Vec<String>,            // OAuth scopes granted
    pub follower_count: u32,
    pub has_profile_pic: bool,
    pub has_banner_pic: bool,
    pub verified: bool,
    pub qualifies: bool,                // Computed eligibility flag
    pub checked_at: u64,                // Unix timestamp
    pub version: u8,                    // Schema version = 2
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthState {
    pub telegram_user_id: u64,
    pub telegram_username: String,
    pub verifier: String,
    pub nonce: String,
    pub created_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TwitterOAuthPayload {
    pub code: String,
    pub state: String,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TwitterProfile {
    pub id: String,
    pub username: String,
    pub name: String,
    pub profile_image_url: Option<String>,
    pub verified: Option<bool>,
    pub public_metrics: Option<TwitterPublicMetrics>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TwitterPublicMetrics {
    pub followers_count: u32,
    pub following_count: u32,
    pub tweet_count: u32,
    pub listed_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TwitterTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub scope: String,
    pub expires_in: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TwitterApiResponse<T> {
    pub data: T,
}

impl TwitterUserV2 {
    pub fn compute_qualifies(&self) -> bool {
        self.follower_count >= 50 
            && self.has_profile_pic 
            && self.has_banner_pic 
            && !self.verified
    }
} 