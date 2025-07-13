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
    view::ViewRequest,
};
use axum::{
    Extension,
    extract::{Json, State},
    http::StatusCode,
};
use quark_core::helpers::dto::{CreateGroupResponse, GroupPayload};

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
    Extension(group): Extension<GroupPayload>,
) -> Result<Json<CreateGroupResponse>, ErrorServer> {
    let group_id = group.group_id;

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

    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(contract_address, "group".to_string()),
        "create_group".to_string(),
        vec![],
        vec![group_id.clone().into()],
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

    let resource_account_address = node
        .view_function(ViewRequest {
            function: format!("{}::group::get_group_account", contract_address),
            type_arguments: vec![],
            arguments: vec![group_id.clone().into()],
        })
        .await
        .map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?
        .into_inner();

    let resource_account_address = resource_account_address.as_str().ok_or(ErrorServer {
        status: StatusCode::NOT_FOUND.into(),
        message: "Resource account address not found".to_string(),
    })?;

    Ok(Json(CreateGroupResponse {
        resource_account_address: resource_account_address.to_string(),
    }))
}
