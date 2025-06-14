use crate::info;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(info::handler::info), components(schemas(info::dto::Info)))]
pub struct ApiDoc;
