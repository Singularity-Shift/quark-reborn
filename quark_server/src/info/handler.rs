use axum::response::Json;

use super::dto::Info;

#[utoipa::path(
    get,
    path = "/",
    summary = "Get information about the Keeper service",
    description = "Returns the version of the Keeper service",
    responses(
        (status = 200, description = "Information about the Keeper service", body = [Info])
    )
)]
#[axum::debug_handler]
pub async fn info() -> Json<Info> {
    // Replace the following with your actual implementation
    let version = env!("CARGO_PKG_VERSION");

    let info = Info::from(("Quark Server".to_string(), version.to_string()));

    Json(info)
}
