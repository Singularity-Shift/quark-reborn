use std::env;

use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_redoc::{Redoc, Servable};

use crate::{docs::dto::ApiDoc, info::handler::info};

pub async fn router() -> Router {
    let network = env::var("APTOS_NETWORK").expect("APTOS_NETWORK environment variable not set");

    let doc = ApiDoc::openapi();

    Router::new()
        .merge(Redoc::with_url("/redoc", doc))
        .route("/", get(info))
        .layer(TraceLayer::new_for_http())
}
