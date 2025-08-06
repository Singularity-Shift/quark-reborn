use aptos_rust_sdk_types::api_types::{
    module_id::ModuleId,
    transaction::{EntryFunction, TransactionPayload},
};
use axum::{
    Extension,
    extract::{Json, State},
    http::StatusCode,
};
use std::{env, sync::Arc};

use crate::{
    admin::handler::{get_admin, get_reviewer_priv_acc},
    error::ErrorServer,
    state::ServerState,
    util::execute_transaction,
};
use quark_core::helpers::dto::{GroupPayload, TransactionResponse};

#[utoipa::path(
    post,
    path = "/migrate-group-id",
    description = "Migrate group id",
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
        (status = 500, description = "Internal Server Error"),
        (status = 401, description = "Unauthorized"),
    )
)]
#[axum::debug_handler]
pub async fn migrate_group_id(
    State(server_state): State<Arc<ServerState>>,
    Extension(group): Extension<GroupPayload>,
) -> Result<Json<TransactionResponse>, ErrorServer> {
    let account_seed: String = env::var("ACCOUNT_SEED").map_err(|e| ErrorServer {
        status: StatusCode::INTERNAL_SERVER_ERROR.into(),
        message: e.to_string(),
    })?;

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

    let group_id = group.group_id;

    // Extract the old group ID from the new format: [group_id]-quark-ai
    let old_group_id = if group_id.ends_with(format!("-{}", account_seed).as_str()) {
        let prefix = group_id.trim_end_matches(format!("-{}", account_seed).as_str());
        prefix.to_string()
    } else {
        return Err(ErrorServer {
            status: StatusCode::BAD_REQUEST.into(),
            message: "Invalid group ID format".to_string(),
        });
    };

    // Call the smart contract function to migrate the group ID
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(contract_address, "group".to_string()),
        "migrate_group_id".to_string(),
        vec![],
        vec![
            bcs::to_bytes(&old_group_id).map_err(|e| ErrorServer {
                status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                message: e.to_string(),
            })?,
            bcs::to_bytes(&group_id).map_err(|e| ErrorServer {
                status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                message: e.to_string(),
            })?,
        ],
    ));

    let response = execute_transaction(
        &node,
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

    Ok(Json(TransactionResponse {
        hash: response.hash,
    }))
}
