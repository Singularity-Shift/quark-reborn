use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_rust_sdk::client::rest_api::AptosFullnodeClient;
use aptos_rust_sdk_types::{
    api_types::{
        address::AccountAddress,
        chain_id::ChainId,
        transaction::{
            GenerateSigningMessage, RawTransaction, RawTransactionWithData, SignedTransaction,
            TransactionPayload,
        },
        transaction_authenticator::{AccountAuthenticator, TransactionAuthenticator},
    },
    state::State,
};
use axum::http::StatusCode;
use quark_core::helpers::dto::{GasPrice, SimulateTransactionResponse, TransactionResponse};

use crate::error::ErrorServer;

pub async fn execute_transaction(
    node: &AptosFullnodeClient,
    admin: AccountAddress,
    reviewer: AccountAddress,
    signer: &Ed25519PrivateKey,
    reviewer_signer: &Ed25519PrivateKey,
    payload: TransactionPayload,
    state: &State,
    chain_id: ChainId,
) -> Result<TransactionResponse, ErrorServer> {
    let resource = node
        .get_account_resources(admin.to_string())
        .await
        .map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?
        .into_inner();

    let sequence_number = resource
        .iter()
        .find(|r| r.type_ == "0x1::account::Account")
        .ok_or(ErrorServer {
            status: StatusCode::NOT_FOUND.into(),
            message: "Account resource not found".to_string(),
        })?
        .data
        .get("sequence_number")
        .ok_or(ErrorServer {
            status: StatusCode::NOT_FOUND.into(),
            message: "Sequence number not found".to_string(),
        })?
        .as_str()
        .ok_or(ErrorServer {
            status: StatusCode::NOT_FOUND.into(),
            message: "Sequence number not found".to_string(),
        })?
        .parse::<u64>()
        .map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?;

    let gas_price = node
        .get_estimate_gas_price()
        .await
        .map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?;

    let gas_price = gas_price.into_inner();

    let max_gas_amount = 100000;
    let gas_price = serde_json::from_value::<GasPrice>(gas_price).map_err(|e| ErrorServer {
        status: StatusCode::INTERNAL_SERVER_ERROR.into(),
        message: e.to_string(),
    })?;

    let gas_unit_price = gas_price.gas_estimate;

    let expiration_timestamp_secs = state.timestamp_usecs / 1000 / 1000 + 60 * 10;

    let raw_transaction = RawTransactionWithData::new_multi_agent(
        RawTransaction::new(
            admin,
            sequence_number,
            payload,
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            chain_id,
        ),
        vec![reviewer],
    );

    let message = raw_transaction
        .generate_signing_message()
        .map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?;

    let signature = signer.sign_message(&message);

    let reviewer_signature = reviewer_signer.sign_message(&message);

    println!("Start simulate transaction");

    let simulate_transaction = node
        .simulate_transaction(SignedTransaction::new(
            raw_transaction.raw_txn().to_owned(),
            TransactionAuthenticator::multi_agent(
                AccountAuthenticator::no_authenticator(),
                vec![reviewer],
                vec![AccountAuthenticator::no_authenticator()],
            ),
        ))
        .await
        .map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?;

    println!("Simulate transaction: {:?}", simulate_transaction);

    let simulate_transaction_inner = simulate_transaction.into_inner();

    let simulate_transaction_success = if simulate_transaction_inner.is_array() {
        // Handle array response - take the first element
        let array = simulate_transaction_inner
            .as_array()
            .ok_or_else(|| ErrorServer {
                status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                message: "Expected array".to_string(),
            })?;
        let first_result = array.get(0).ok_or_else(|| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: "Empty simulation result array".to_string(),
        })?;
        serde_json::from_value::<SimulateTransactionResponse>(first_result.clone()).map_err(
            |e| ErrorServer {
                status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                message: e.to_string(),
            },
        )?
    } else {
        // Handle single object response
        serde_json::from_value::<SimulateTransactionResponse>(simulate_transaction_inner.clone())
            .map_err(|e| ErrorServer {
                status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                message: e.to_string(),
            })?
    };

    if !simulate_transaction_success.success {
        return Err(ErrorServer {
            status: StatusCode::BAD_REQUEST.into(),
            message: format!(
                "Simulate transaction failed: {}",
                simulate_transaction_success.vm_status
            )
            .into(),
        });
    }

    let transaction = node
        .submit_transaction(SignedTransaction::new(
            raw_transaction.raw_txn().to_owned(),
            TransactionAuthenticator::multi_agent(
                AccountAuthenticator::ed25519(Ed25519PublicKey::from(signer), signature),
                vec![reviewer],
                vec![AccountAuthenticator::ed25519(
                    Ed25519PublicKey::from(reviewer_signer),
                    reviewer_signature,
                )],
            ),
        ))
        .await
        .map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?
        .into_inner();

    println!("Transaction: {:?}", transaction);

    let pay_users_response: TransactionResponse =
        serde_json::from_value(transaction).map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?;

    Ok(pay_users_response)
}
