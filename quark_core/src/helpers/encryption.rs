use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::{
    aead::{self, Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM},
    rand::{SecureRandom, SystemRandom},
};

const NONCE_LEN: usize = 12; // AES-GCM nonce length

/// Encrypt a token using AES-256-GCM
pub fn encrypt_token(token: &str, key: &str) -> Result<String> {
    // Derive a 32-byte key from the provided key string
    let key_bytes = derive_key(key)?;
    
    // Create the encryption key
    let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map_err(|_| anyhow::anyhow!("Failed to create encryption key"))?;
    let less_safe_key = LessSafeKey::new(unbound_key);
    
    // Generate a random nonce
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| anyhow::anyhow!("Failed to generate nonce"))?;
    
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    
    // Encrypt the token
    let mut token_bytes = token.as_bytes().to_vec();
    less_safe_key
        .seal_in_place_append_tag(nonce, Aad::empty(), &mut token_bytes)
        .map_err(|_| anyhow::anyhow!("Encryption failed"))?;
    
    // Combine nonce and encrypted data
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&token_bytes);
    
    // Base64 encode the result
    Ok(URL_SAFE_NO_PAD.encode(&result))
}

/// Decrypt a token using AES-256-GCM
pub fn decrypt_token(encrypted_token: &str, key: &str) -> Result<String> {
    // Decode from base64
    let encrypted_data = URL_SAFE_NO_PAD
        .decode(encrypted_token)
        .map_err(|_| anyhow::anyhow!("Invalid base64 encoding"))?;
    
    if encrypted_data.len() < NONCE_LEN {
        return Err(anyhow::anyhow!("Invalid encrypted data length"));
    }
    
    // Split nonce and encrypted content
    let (nonce_bytes, encrypted_content) = encrypted_data.split_at(NONCE_LEN);
    let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
        .map_err(|_| anyhow::anyhow!("Invalid nonce"))?;
    
    // Derive the same key
    let key_bytes = derive_key(key)?;
    
    // Create the decryption key
    let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map_err(|_| anyhow::anyhow!("Failed to create decryption key"))?;
    let less_safe_key = LessSafeKey::new(unbound_key);
    
    // Decrypt the content
    let mut content = encrypted_content.to_vec();
    let decrypted_data = less_safe_key
        .open_in_place(nonce, Aad::empty(), &mut content)
        .map_err(|_| anyhow::anyhow!("Decryption failed"))?;
    
    // Convert to string
    String::from_utf8(decrypted_data.to_vec())
        .map_err(|_| anyhow::anyhow!("Invalid UTF-8 in decrypted data"))
}

/// Derive a 32-byte key from a string using PBKDF2-like approach
fn derive_key(key_str: &str) -> Result<[u8; 32]> {
    use ring::digest::{digest, SHA256};
    
    // For simplicity, we'll use SHA256 hash of the key string
    // In production, consider using PBKDF2 with a proper salt
    let hash = digest(&SHA256, key_str.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(hash.as_ref());
    Ok(key)
}

/// Generate a secure random encryption key (for initial setup)
pub fn generate_encryption_key() -> Result<String> {
    let rng = SystemRandom::new();
    let mut key_bytes = [0u8; 32];
    rng.fill(&mut key_bytes)
        .map_err(|_| anyhow::anyhow!("Failed to generate random key"))?;
    
    Ok(URL_SAFE_NO_PAD.encode(&key_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_roundtrip() {
        let token = "test_access_token_12345";
        let key = "test_encryption_key";
        
        let encrypted = encrypt_token(token, key).unwrap();
        assert_ne!(encrypted, token);
        
        let decrypted = decrypt_token(&encrypted, key).unwrap();
        assert_eq!(decrypted, token);
    }

    #[test]
    fn test_encryption_different_keys() {
        let token = "test_token";
        let key1 = "key1";
        let key2 = "key2";
        
        let encrypted = encrypt_token(token, key1).unwrap();
        let result = decrypt_token(&encrypted, key2);
        
        // Should fail with wrong key
        assert!(result.is_err());
    }

    #[test]
    fn test_key_generation() {
        let key1 = generate_encryption_key().unwrap();
        let key2 = generate_encryption_key().unwrap();
        
        // Keys should be different
        assert_ne!(key1, key2);
        
        // Keys should be valid base64
        assert!(URL_SAFE_NO_PAD.decode(&key1).is_ok());
        assert!(URL_SAFE_NO_PAD.decode(&key2).is_ok());
    }
} 