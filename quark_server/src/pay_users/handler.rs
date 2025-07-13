use std::{str::FromStr, sync::Arc};

use aptos_rust_sdk_types::api_types::{
    address::AccountAddress,
    module_id::ModuleId,
    transaction::{EntryFunction, TransactionPayload},
    type_tag::TypeTag,
};
use axum::{
    Extension,
    extract::{Json, State},
    http::StatusCode,
};

use crate::{
    admin::handler::{get_admin, get_reviewer_priv_acc},
    error::ErrorServer,
    state::ServerState,
    util::execute_transaction,
};
use quark_core::helpers::dto::{
    PayUsersRequest, PayUsersVersion, TransactionResponse, UserPayload,
};

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
) -> Result<Json<TransactionResponse>, ErrorServer> {
    let (admin, signer) = get_admin().map_err(|e| ErrorServer {
        status: StatusCode::INTERNAL_SERVER_ERROR.into(),
        message: e.to_string(),
    })?;

    let (reviewer, reviewer_signer) = get_reviewer_priv_acc().map_err(|e| ErrorServer {
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
                ModuleId::new(contract_address, "user_v5".to_string()),
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
            ModuleId::new(contract_address, "user_v5".to_string()),
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

    let pay_users_response = execute_transaction(
        node,
        admin,
        reviewer,
        &signer,
        &reviewer_signer,
        payload,
        &state,
        chain_id,
    )
    .await
    .map_err(|e| ErrorServer {
        status: StatusCode::INTERNAL_SERVER_ERROR.into(),
        message: e.to_string(),
    })?;

    Ok(Json(pay_users_response))
}
