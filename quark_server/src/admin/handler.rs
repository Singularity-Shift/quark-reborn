use std::env;

use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_rust_sdk_types::api_types::{
    address::AccountAddress, transaction_authenticator::AuthenticationKey,
};

use crate::error::ErrorServer;

pub fn get_admin() -> Result<(AccountAddress, Ed25519PrivateKey), ErrorServer> {
    let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY environment variable not set");
    let private_key = private_key.trim_matches('"').trim_start_matches("0x");

    let mut seed = [0u8; 32];
    let hex_bytes = hex::decode(&private_key).unwrap();

    seed[..hex_bytes.len()].copy_from_slice(&hex_bytes);

    let private_key = Ed25519PrivateKey::try_from(hex_bytes.as_slice()).unwrap();

    let auth_key = AuthenticationKey::ed25519(&Ed25519PublicKey::from(&private_key));

    let admin = auth_key.account_address();

    Ok((admin, private_key))
}
