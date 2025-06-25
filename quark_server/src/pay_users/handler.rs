use std::{str::FromStr, sync::Arc};

use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_rust_sdk_types::api_types::{
    address::AccountAddress,
    module_id::ModuleId,
    transaction::{
        EntryFunction, GenerateSigningMessage, RawTransaction, SignedTransaction,
        TransactionPayload,
    },
    transaction_authenticator::{AccountAuthenticator, TransactionAuthenticator},
    type_tag::TypeTag,
};
use axum::{
    Extension,
    extract::{Json, State},
    http::StatusCode,
};

use crate::{admin::handler::get_admin, error::ErrorServer, state::ServerState};
use quark_core::helpers::dto::{PayUsersRequest, PayUsersResponse, PayUsersVersion, UserPayload};

#[utoipa::path(
    post,
    path = "/pay-users",
    request_body = [PayUsersRequest],
    description = "Pay users",
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
    )
)]
#[axum::debug_handler]
pub async fn pay_users(
    State(server_state): State<Arc<ServerState>>,
    Extension(user): Extension<UserPayload>,
    Json(request): Json<PayUsersRequest>,
) -> Result<Json<PayUsersResponse>, ErrorServer> {
    let (admin, signer) = get_admin().map_err(|e| ErrorServer {
        status: StatusCode::INTERNAL_SERVER_ERROR.into(),
        message: e.to_string(),
    })?;

    let node = server_state.node();
    let chain_id = server_state.chain_id();
    let contract_address = server_state.contract_address();
    let state = node.get_state().await.map_err(|e| ErrorServer {
        status: StatusCode::INTERNAL_SERVER_ERROR.into(),
        message: e.to_string(),
    })?;

    let account_address =
        AccountAddress::from_str(&user.account_address).map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?;

    let amount = request.amount;
    let users = request.users;
    let version = request.version;
    let coin_type = request.coin_type;
    let users = users
        .iter()
        .map(|u| {
            AccountAddress::from_str(u).map_err(|e| ErrorServer {
                status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                message: e.to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let payload = match version {
        PayUsersVersion::V1 => {
            let token_type = TypeTag::from_str(&coin_type).map_err(|e| ErrorServer {
                status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                message: e.to_string(),
            })?;

            TransactionPayload::EntryFunction(EntryFunction::new(
                ModuleId::new(contract_address, "user_v3".to_string()),
                "pay_to_users_v1".to_string(),
                vec![token_type],
                vec![
                    account_address.to_vec(),
                    amount.to_le_bytes().to_vec(),
                    bcs::to_bytes(&users).map_err(|e| ErrorServer {
                        status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                        message: e.to_string(),
                    })?,
                ],
            ))
        }
        PayUsersVersion::V2 => TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(contract_address, "user_v3".to_string()),
            "pay_to_users_v2".to_string(),
            vec![],
            vec![
                account_address.to_vec(),
                amount.to_le_bytes().to_vec(),
                AccountAddress::from_str(&coin_type)
                    .map_err(|e| ErrorServer {
                        status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                        message: e.to_string(),
                    })?
                    .to_vec(),
                bcs::to_bytes(&users).map_err(|e| ErrorServer {
                    status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                    message: e.to_string(),
                })?,
            ],
        )),
    };

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

    let max_gas_amount = 1500;
    let gas_unit_price = 100;
    let expiration_timestamp_secs = state.timestamp_usecs / 1000 / 1000 + 60 * 10;

    let raw_transaction = RawTransaction::new(
        admin,
        sequence_number,
        payload,
        max_gas_amount,
        gas_unit_price,
        expiration_timestamp_secs,
        chain_id,
    );

    let message = raw_transaction
        .generate_signing_message()
        .map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?;

    let signature = signer.sign_message(&message);

    let simulate_transaction = node
        .simulate_transaction(SignedTransaction::new(
            raw_transaction.clone(),
            TransactionAuthenticator::single_sender(AccountAuthenticator::no_authenticator()),
        ))
        .await
        .map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?;

    println!("Simulate Transaction: {:?}", simulate_transaction);

    let transaction = node
        .submit_transaction(SignedTransaction::new(
            raw_transaction,
            TransactionAuthenticator::ed25519(Ed25519PublicKey::from(&signer), signature),
        ))
        .await
        .map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?
        .into_inner();

    let pay_users_response: PayUsersResponse =
        serde_json::from_value(transaction).map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?;

    Ok(Json(pay_users_response))
}
