use crate::{info, pay_users};
use quark_core::helpers::dto::{PayUsersRequest, PurchaseRequest};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(info::handler::info, pay_users::handler::pay_users,),
    components(schemas(info::dto::Info, PayUsersRequest, PurchaseRequest))
)]
pub struct ApiDoc;
