use std::str::FromStr;

use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_rust_sdk_types::api_types::{
    address::AccountAddress,
    module_id::ModuleId,
    transaction::{
        EntryFunction, GenerateSigningMessage, RawTransaction, RawTransactionWithData,
        SignedTransaction, TransactionPayload,
    },
    transaction_authenticator::{AccountAuthenticator, TransactionAuthenticator},
    type_tag::TypeTag,
};
use quark_core::helpers::dto::{CoinVersion, SimulateTransactionResponse, TransactionResponse};
use serde_json;

use crate::{
    ConsumerResult,
    admin::handler::{get_admin, get_reviewer_priv_acc},
    error::ConsumerError,
    purchase::dto::{Purchase, PurchaseType},
};

pub async fn purchase_ai(purchase: Purchase) -> ConsumerResult<TransactionResponse> {
    let (admin, signer) = get_admin().map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?;

    let (reviewer, reviewer_signer) =
        get_reviewer_priv_acc().map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?;

    let node = purchase.node;
    let token_address = purchase.token_address;
    let coin_version = purchase.coin_version;
    let contract_address = purchase.contract_address;
    let amount = purchase.amount;
    let purchase_type = purchase.purchase_type;
    let chain_id = purchase.chain_id;
    let state = node
        .get_state()
        .await
        .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?;

    let token_type = TypeTag::from_str(token_address.to_string().as_str())
        .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?;

    let payload = match purchase_type {
        PurchaseType::User(account_address) => {
            let user_address = AccountAddress::from_str(account_address.as_str())
                .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?;

            let payload = match coin_version {
                CoinVersion::V1 => TransactionPayload::EntryFunction(EntryFunction::new(
                    ModuleId::new(contract_address, "user".to_string()),
                    "pay_ai".to_string(),
                    vec![token_type],
                    vec![user_address.to_vec(), amount.to_le_bytes().to_vec()],
                )),
                CoinVersion::V2 => TransactionPayload::EntryFunction(EntryFunction::new(
                    ModuleId::new(contract_address, "user".to_string()),
                    "pay_ai_v2".to_string(),
                    vec![],
                    vec![
                        user_address.to_vec(),
                        amount.to_le_bytes().to_vec(),
                        AccountAddress::from_str(&token_address)
                            .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?
                            .to_vec(),
                    ],
                )),
            };

            payload
        }
        PurchaseType::Group(group_id) => {
            let payload = match coin_version {
                CoinVersion::V1 => TransactionPayload::EntryFunction(EntryFunction::new(
                    ModuleId::new(contract_address, "group".to_string()),
                    "pay_ai".to_string(),
                    vec![token_type],
                    vec![
                        bcs::to_bytes(&group_id)
                            .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?,
                        amount.to_le_bytes().to_vec(),
                    ],
                )),
                CoinVersion::V2 => TransactionPayload::EntryFunction(EntryFunction::new(
                    ModuleId::new(contract_address, "group".to_string()),
                    "pay_ai_v2".to_string(),
                    vec![],
                    vec![
                        bcs::to_bytes(&group_id)
                            .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?,
                        amount.to_le_bytes().to_vec(),
                        AccountAddress::from_str(&token_address)
                            .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?
                            .to_vec(),
                    ],
                )),
            };

            payload
        }
    };

    let resource = node
        .get_account_resources(admin.to_string())
        .await
        .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?
        .into_inner();

    let sequence_number = resource
        .iter()
        .find(|r| r.type_ == "0x1::account::Account")
        .ok_or(ConsumerError::InvalidMessage(
            "Account resource not found".to_string(),
        ))?
        .data
        .get("sequence_number")
        .ok_or(ConsumerError::InvalidMessage(
            "Sequence number not found".to_string(),
        ))?
        .as_str()
        .ok_or(ConsumerError::InvalidMessage(
            "Sequence number not found".to_string(),
        ))?
        .parse::<u64>()
        .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?;

    let max_gas_amount = 1500;
    let gas_unit_price = 100;
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
        .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?;

    let signature = signer.sign_message(&message);

    let reviewer_signature = reviewer_signer.sign_message(&message);

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
        .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?;

    let simulate_transaction_inner = simulate_transaction.into_inner();

    let simulate_transaction_success = if simulate_transaction_inner.is_array() {
        // Handle array response - take the first element
        let array = simulate_transaction_inner
            .as_array()
            .ok_or_else(|| ConsumerError::InvalidMessage("Expected array".to_string()))?;
        let first_result = array.get(0).ok_or_else(|| {
            ConsumerError::InvalidMessage("Empty simulation result array".to_string())
        })?;
        serde_json::from_value::<SimulateTransactionResponse>(first_result.clone())
            .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?
    } else {
        // Handle single object response
        serde_json::from_value::<SimulateTransactionResponse>(simulate_transaction_inner.clone())
            .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?
    };

    if !simulate_transaction_success.success {
        return Err(ConsumerError::InvalidMessage(format!(
            "Simulate transaction failed: {}",
            simulate_transaction_success.vm_status
        )));
    }

    let transaction = node
        .submit_transaction(SignedTransaction::new(
            raw_transaction.raw_txn().to_owned(),
            TransactionAuthenticator::multi_agent(
                AccountAuthenticator::ed25519(Ed25519PublicKey::from(&signer), signature),
                vec![reviewer],
                vec![AccountAuthenticator::ed25519(
                    Ed25519PublicKey::from(&reviewer_signer),
                    reviewer_signature,
                )],
            ),
        ))
        .await
        .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?
        .into_inner();

    println!("Transaction: {:?}", transaction);

    let transaction_response: TransactionResponse = serde_json::from_value(transaction)
        .map_err(|e| ConsumerError::InvalidMessage(e.to_string()))?;

    Ok(transaction_response)
}
