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

    let seed = hex::encode(seed);

    let hex_bytes = hex::decode(&seed).unwrap();

    let private_key = Ed25519PrivateKey::try_from(hex_bytes.as_slice()).unwrap();

    let auth_key = AuthenticationKey::ed25519(&Ed25519PublicKey::from(&private_key));

    let admin = auth_key.account_address();

    Ok((admin, private_key))
}
