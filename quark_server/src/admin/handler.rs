use std::env;

use aptos_crypto::{
    ValidCryptoMaterialStringExt,
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
};
use aptos_rust_sdk_types::api_types::{
    address::AccountAddress, transaction_authenticator::AuthenticationKey,
};

use crate::error::ErrorServer;

pub fn get_admin() -> Result<(AccountAddress, Ed25519PrivateKey), ErrorServer> {
    let seed = env::var("SEED").expect("SEED environment variable not set");
    let hex_seed = hex::encode(seed.into_bytes());

    let private_key = Ed25519PrivateKey::from_encoded_string(&hex_seed).unwrap();

    let auth_key = AuthenticationKey::ed25519(&Ed25519PublicKey::from(&private_key));

    let admin = auth_key.account_address();

    Ok((admin, private_key))
}
