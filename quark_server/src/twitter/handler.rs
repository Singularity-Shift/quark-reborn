use anyhow::Result;
use axum::{extract::State, http::StatusCode, Json, response::Json as ResponseJson};
use log::{debug, error, info, warn};
use quark_core::twitter::{
    auth::{current_timestamp, parse_oauth_state},
    dto::{TwitterApiResponse, TwitterProfile, TwitterPublicMetrics, TwitterTokenResponse, TwitterUserV2},
    storage::TwitterStorage,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, env, sync::Arc};

use crate::state::ServerState;

#[derive(Debug, Serialize, Deserialize)]
pub struct TwitterOAuthRequest {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TwitterOAuthResponse {
    pub success: bool,
    pub user: Option<TwitterUserResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TwitterUserResponse {
    pub telegram_username: String,
    pub twitter_handle: String,
    pub twitter_id: String,
    pub follower_count: u32,
    pub qualifies: bool,
}

/// Handler for Twitter OAuth callback
pub async fn twitter_oauth_callback(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<TwitterOAuthRequest>,
) -> Result<ResponseJson<TwitterOAuthResponse>, StatusCode> {
    info!("Twitter OAuth callback received");
    
    match process_twitter_oauth(payload).await {
        Ok(response) => Ok(ResponseJson(response)),
        Err(e) => {
            error!("Twitter OAuth error: {}", e);
            Ok(ResponseJson(TwitterOAuthResponse {
                success: false,
                user: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

async fn process_twitter_oauth(payload: TwitterOAuthRequest) -> Result<TwitterOAuthResponse> {
    debug!("Processing Twitter OAuth with state: {}", payload.state);
    
    // Get environment variables
    let client_id = env::var("TWITTER_CLIENT_ID")
        .map_err(|_| anyhow::anyhow!("TWITTER_CLIENT_ID not set"))?;
    let client_secret = env::var("TWITTER_CLIENT_SECRET")
        .map_err(|_| anyhow::anyhow!("TWITTER_CLIENT_SECRET not set"))?;
    let redirect_uri = env::var("TWITTER_REDIRECT_URI")
        .map_err(|_| anyhow::anyhow!("TWITTER_REDIRECT_URI not set"))?;
    
    // Get sled database URL
    let sled_url = env::var("SLED_URL")
        .map_err(|_| anyhow::anyhow!("SLED_URL not set"))?;
    let db = sled::open(&sled_url)
        .map_err(|e| anyhow::anyhow!("Failed to open sled database: {}", e))?;
    
    let storage = TwitterStorage::new(db);
    
    // Parse and validate OAuth state
    let (telegram_user_id, nonce) = parse_oauth_state(&payload.state)?;
    
    // Retrieve OAuth state from storage
    let oauth_state = storage.get_oauth_state(&payload.state)?
        .ok_or_else(|| anyhow::anyhow!("Invalid or expired OAuth state"))?;
    
    debug!("Found OAuth state for user: {}", oauth_state.telegram_user_id);
    
    // Exchange code for access token
    let token_response = exchange_code_for_token(
        &payload.code,
        &oauth_state.verifier,
        &client_id,
        &client_secret,
        &redirect_uri,
    ).await?;
    
    debug!("Successfully exchanged code for token");
    
    // Fetch user profile
    let profile = fetch_twitter_profile(&token_response.access_token).await?;
    
    debug!("Fetched Twitter profile for: @{}", profile.username);
    
    // Check if user has banner image
    let has_banner = check_profile_banner(&profile.id, &token_response.access_token).await?;
    
    // Create TwitterUserV2 object
    let mut twitter_user = TwitterUserV2 {
        telegram_username: oauth_state.telegram_username.clone(),
        telegram_user_id: oauth_state.telegram_user_id,
        twitter_handle: profile.username.clone(),
        twitter_id: profile.id.parse::<u64>()
            .map_err(|_| anyhow::anyhow!("Invalid Twitter user ID"))?,
        access_token: encrypt_token(&token_response.access_token)?,
        refresh_token: token_response.refresh_token.map(|t| encrypt_token(&t)).transpose()?,
        scopes: token_response.scope.split_whitespace().map(|s| s.to_string()).collect(),
        follower_count: profile.public_metrics
            .as_ref()
            .map(|m| m.followers_count)
            .unwrap_or(0),
        has_profile_pic: profile.profile_image_url.is_some(),
        has_banner_pic: has_banner,
        verified: profile.verified.unwrap_or(false),
        qualifies: false, // Will be computed
        checked_at: current_timestamp(),
        version: 2,
    };
    
    // Compute qualification status
    twitter_user.qualifies = twitter_user.compute_qualifies();
    
    info!(
        "User @{} qualification: {} (followers: {}, profile_pic: {}, banner: {}, verified: {})",
        twitter_user.twitter_handle,
        twitter_user.qualifies,
        twitter_user.follower_count,
        twitter_user.has_profile_pic,
        twitter_user.has_banner_pic,
        twitter_user.verified
    );
    
    // Store user data
    storage.store_twitter_user(&twitter_user)?;
    
    // Clean up OAuth state
    storage.remove_oauth_state(&payload.state)?;
    
    debug!("Successfully stored Twitter user data");
    
    Ok(TwitterOAuthResponse {
        success: true,
        user: Some(TwitterUserResponse {
            telegram_username: twitter_user.telegram_username,
            twitter_handle: twitter_user.twitter_handle,
            twitter_id: twitter_user.twitter_id.to_string(),
            follower_count: twitter_user.follower_count,
            qualifies: twitter_user.qualifies,
        }),
        error: None,
    })
}

async fn exchange_code_for_token(
    code: &str,
    verifier: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> Result<TwitterTokenResponse> {
    let client = Client::new();
    
    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("code", code);
    params.insert("redirect_uri", redirect_uri);
    params.insert("code_verifier", verifier);
    params.insert("client_id", client_id);
    
    // Create basic auth header
    let auth_string = format!("{}:{}", client_id, client_secret);
    let auth_header = format!("Basic {}", base64::encode(auth_string));
    
    let response = client
        .post("https://api.twitter.com/2/oauth2/token")
        .header("Authorization", auth_header)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Token exchange failed: {}", error_text));
    }
    
    let token_response: TwitterTokenResponse = response.json().await?;
    Ok(token_response)
}

async fn fetch_twitter_profile(access_token: &str) -> Result<TwitterProfile> {
    let client = Client::new();
    
    let response = client
        .get("https://api.twitter.com/2/users/me")
        .header("Authorization", format!("Bearer {}", access_token))
        .query(&[
            ("user.fields", "id,username,name,profile_image_url,verified,public_metrics"),
        ])
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Profile fetch failed: {}", error_text));
    }
    
    let api_response: TwitterApiResponse<TwitterProfile> = response.json().await?;
    Ok(api_response.data)
}

async fn check_profile_banner(user_id: &str, access_token: &str) -> Result<bool> {
    let client = Client::new();
    
    // Use the v1.1 API for banner info
    let response = client
        .get(&format!("https://api.twitter.com/1.1/users/profile_banner.json"))
        .header("Authorization", format!("Bearer {}", access_token))
        .query(&[("user_id", user_id)])
        .send()
        .await?;
    
    // If the request succeeds, the user has a banner
    // If it returns 404, the user doesn't have a banner
    match response.status().as_u16() {
        200 => Ok(true),
        404 => Ok(false),
        _ => {
            warn!("Unexpected status code when checking banner: {}", response.status());
            // Default to false if we can't determine
            Ok(false)
        }
    }
}

fn encrypt_token(token: &str) -> Result<String> {
    use quark_core::helpers::encryption;
    
    // Get encryption key from environment
    let key = env::var("TW_TOKEN_KEY")
        .map_err(|_| anyhow::anyhow!("TW_TOKEN_KEY not set"))?;
    
    encryption::encrypt_token(token, &key)
}

fn decrypt_token(encrypted_token: &str) -> Result<String> {
    use quark_core::helpers::encryption;
    
    let key = env::var("TW_TOKEN_KEY")
        .map_err(|_| anyhow::anyhow!("TW_TOKEN_KEY not set"))?;
    
    encryption::decrypt_token(encrypted_token, &key)
} 