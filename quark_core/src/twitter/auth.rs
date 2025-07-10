use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a PKCE code verifier and challenge pair
pub fn generate_pkce_pair() -> (String, String) {
    // Generate 128 bytes of random data for the verifier
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..128).map(|_| rng.gen()).collect();
    
    // Base64 URL-safe encode the verifier
    let verifier = URL_SAFE_NO_PAD.encode(&random_bytes);
    
    // Create SHA256 hash of the verifier
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    
    // Base64 URL-safe encode the challenge
    let challenge = URL_SAFE_NO_PAD.encode(&hash);
    
    (verifier, challenge)
}

/// Generate a cryptographically secure nonce
pub fn generate_nonce() -> String {
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    URL_SAFE_NO_PAD.encode(&random_bytes)
}

/// Build Twitter OAuth2 authorization URL
pub fn build_auth_url(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
) -> String {
    format!(
        "https://twitter.com/i/oauth2/authorize\
         ?response_type=code\
         &client_id={}\
         &redirect_uri={}\
         &scope={}\
         &state={}\
         &code_challenge={}\
         &code_challenge_method=S256",
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode("tweet.read users.read"),
        urlencoding::encode(state),
        urlencoding::encode(code_challenge)
    )
}

/// Create OAuth state string
pub fn create_oauth_state(telegram_user_id: u64, nonce: &str) -> String {
    format!("{}|{}", telegram_user_id, nonce)
}

/// Parse OAuth state string
pub fn parse_oauth_state(state: &str) -> Result<(u64, String)> {
    let parts: Vec<&str> = state.split('|').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid state format"));
    }
    
    let telegram_user_id = parts[0].parse::<u64>()?;
    let nonce = parts[1].to_string();
    
    Ok((telegram_user_id, nonce))
}

/// Get current Unix timestamp
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Check if OAuth state has expired (15 minutes TTL)
pub fn is_state_expired(created_at: u64) -> bool {
    let now = current_timestamp();
    let ttl_seconds = 15 * 60; // 15 minutes
    (now - created_at) > ttl_seconds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let (verifier, challenge) = generate_pkce_pair();
        assert!(!verifier.is_empty());
        assert!(!challenge.is_empty());
        assert_ne!(verifier, challenge);
    }

    #[test]
    fn test_state_operations() {
        let user_id = 12345u64;
        let nonce = generate_nonce();
        let state = create_oauth_state(user_id, &nonce);
        
        let (parsed_user_id, parsed_nonce) = parse_oauth_state(&state).unwrap();
        assert_eq!(user_id, parsed_user_id);
        assert_eq!(nonce, parsed_nonce);
    }

    #[test]
    fn test_auth_url_building() {
        let url = build_auth_url(
            "test_client_id",
            "https://example.com/callback",
            "test_state",
            "test_challenge"
        );
        
        assert!(url.contains("twitter.com/i/oauth2/authorize"));
        assert!(url.contains("client_id=test_client_id"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("state=test_state"));
        assert!(url.contains("code_challenge=test_challenge"));
    }
} 