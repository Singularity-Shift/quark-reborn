use std::sync::Arc;

use crate::{
    admin::handler::{get_admin, get_reviewer_priv_acc},
    error::ErrorServer,
    state::ServerState,
    util::execute_transaction,
};
use aptos_rust_sdk_types::api_types::{
    module_id::ModuleId,
    transaction::{EntryFunction, TransactionPayload},
};
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use quark_core::helpers::dto::CreateGroupRequest;

#[utoipa::path(
    post,
    path = "/create-group",
    description = "Create group",
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
    )
)]
pub async fn create_group(
    State(server_state): State<Arc<ServerState>>,
    Json(request): Json<CreateGroupRequest>,
) -> Result<Json<()>, ErrorServer> {
    let group_id = request.group_id;

    let group_id = group_id.to_string();

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

    println!("Creating group: {}", group_id);

    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(contract_address, "group_v5".to_string()),
        "create_group".to_string(),
        vec![],
        vec![bcs::to_bytes(&group_id).map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?],
    ));

    execute_transaction(
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

    Ok(Json(()))
}
