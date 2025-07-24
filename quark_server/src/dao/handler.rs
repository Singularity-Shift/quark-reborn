use std::{str::FromStr, sync::Arc};

use aptos_rust_sdk_types::api_types::{
    address::AccountAddress,
    module_id::ModuleId,
    transaction::{EntryFunction, TransactionPayload},
    type_tag::TypeTag,
};
use axum::{Json, extract::State, http::StatusCode};
use quark_core::helpers::dto::{CoinVersion, CreateDaoRequest, TransactionResponse};

use crate::{
    admin::handler::{get_admin, get_reviewer_priv_acc},
    error::ErrorServer,
    state::ServerState,
    util::execute_transaction,
};

use chrono::Utc;

#[utoipa::path(
    post,
    path = "/dao",
    request_body = [CreateDaoRequest],
    description = "Create a new DAO",
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
    )
)]
pub async fn create_dao(
    State(server_state): State<Arc<ServerState>>,
    Json(request): Json<CreateDaoRequest>,
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

    let group_id = request.group_id;

    let start_date = request.start_date;
    let end_date = request.end_date;

    let now = Utc::now().timestamp();

    if start_date < now as u64 {
        return Err(ErrorServer {
            status: StatusCode::BAD_REQUEST.into(),
            message: "Start date must be in the future".to_string(),
        });
    }

    if end_date < start_date {
        return Err(ErrorServer {
            status: StatusCode::BAD_REQUEST.into(),
            message: "End date must be after start date".to_string(),
        });
    }

    let dao_id = request.dao_id;

    let version = request.version;

    let currency = request.currency;

    let options = request.options;

    let payload = match version {
        CoinVersion::V1 => {
            let coin_type = TypeTag::from_str(&currency).map_err(|e| ErrorServer {
                status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                message: e.to_string(),
            })?;

            TransactionPayload::EntryFunction(EntryFunction::new(
                ModuleId::new(contract_address, "group".to_string()),
                "create_group_dao_v1".to_string(),
                vec![coin_type],
                vec![
                    bcs::to_bytes(&group_id).map_err(|e| ErrorServer {
                        status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                        message: e.to_string(),
                    })?,
                    bcs::to_bytes(&dao_id).map_err(|e| ErrorServer {
                        status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                        message: e.to_string(),
                    })?,
                    bcs::to_bytes(&options).map_err(|e| ErrorServer {
                        status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                        message: e.to_string(),
                    })?,
                    start_date.to_le_bytes().to_vec(),
                    end_date.to_le_bytes().to_vec(),
                ],
            ))
        }
        CoinVersion::V2 => TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(contract_address, "group".to_string()),
            "create_group_dao_v2".to_string(),
            vec![],
            vec![
                bcs::to_bytes(&group_id).map_err(|e| ErrorServer {
                    status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                    message: e.to_string(),
                })?,
                bcs::to_bytes(&dao_id).map_err(|e| ErrorServer {
                    status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                    message: e.to_string(),
                })?,
                bcs::to_bytes(&options).map_err(|e| ErrorServer {
                    status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                    message: e.to_string(),
                })?,
                AccountAddress::from_str(&currency)
                    .map_err(|e| ErrorServer {
                        status: StatusCode::INTERNAL_SERVER_ERROR.into(),
                        message: e.to_string(),
                    })?
                    .to_vec(),
                start_date.to_le_bytes().to_vec(),
                end_date.to_le_bytes().to_vec(),
            ],
        )),
    };

    let result = execute_transaction(
        node,
        admin,
        reviewer,
        &signer,
        &reviewer_signer,
        payload,
        &state,
        chain_id,
    )
    .await?;

    Ok(Json(result))
}
