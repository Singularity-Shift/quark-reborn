use std::sync::Arc;

use axum::{
    Extension,
    extract::{Json, State},
    http::StatusCode,
};
use quark_core::helpers::dto::{PurchaseMessage, PurchaseRequest, UserPayload};
use redis::AsyncCommands;

use crate::{error::ErrorServer, state::ServerState};

#[utoipa::path(
    post,
    path = "/purchase",
    request_body = [PurchaseRequest],
    description = "Purchase",
    responses(
        (status = 200, description = "Success"),
        (status = 400, description = "Bad Request"),
    )
)]
#[axum::debug_handler]
pub async fn purchase(
    State(server_state): State<Arc<ServerState>>,
    Extension(user): Extension<UserPayload>,
    Json(request): Json<PurchaseRequest>,
) -> Result<Json<()>, ErrorServer> {
    let purchase_message: PurchaseMessage = (request, user.account_address).into();

    let message = serde_json::to_string(&purchase_message).unwrap();

    let mut redis_client = server_state.redis_client().clone();

    println!("Purchase message: {}", message);

    let _: () = redis_client
        .lpush("purchase", message)
        .await
        .map_err(|e| ErrorServer {
            status: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e.to_string(),
        })?;

    Ok(Json(()))
}
