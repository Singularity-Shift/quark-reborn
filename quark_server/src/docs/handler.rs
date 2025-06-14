use axum::response::Json;
use utoipa::OpenApi;

use super::dto::ApiDoc;

#[axum::debug_handler]
pub async fn api_docs() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
