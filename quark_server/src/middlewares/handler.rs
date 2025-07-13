use axum::{extract::Request, middleware::Next, response::Response};
use quark_core::helpers::dto::{GroupPayload, UserPayload};
use quark_core::helpers::jwt::JwtManager;

use crate::error::ErrorServer;

pub async fn auth(mut req: Request, next: Next) -> Result<Response, ErrorServer> {
    let headers = req.headers();
    let token = headers.get("Authorization").and_then(|h| h.to_str().ok());

    if let Some(token) = token {
        let jwt_manager = JwtManager::new();
        let token = token.replace("Bearer ", "");
        let claims = jwt_manager
            .validate_token(&token)
            .map_err(|e| ErrorServer {
                message: e.to_string(),
                status: 401,
            })?;

        let account_address = claims.account_address;

        let user = UserPayload { account_address };

        req.extensions_mut().insert(user);
    } else {
        return Err(ErrorServer {
            message: "Unauthorized".to_string(),
            status: 401,
        });
    }

    Ok(next.run(req).await)
}

pub async fn auth_group(mut req: Request, next: Next) -> Result<Response, ErrorServer> {
    let headers = req.headers();
    let token = headers.get("Authorization").and_then(|h| h.to_str().ok());

    if let Some(token) = token {
        let jwt_manager = JwtManager::new();
        let token = token.replace("Bearer ", "");
        let claims = jwt_manager
            .validate_group_token(&token)
            .map_err(|e| ErrorServer {
                message: e.to_string(),
                status: 401,
            })?;

        let group_id = claims.group_id;

        req.extensions_mut().insert(GroupPayload { group_id });
    } else {
        return Err(ErrorServer {
            message: "Unauthorized".to_string(),
            status: 401,
        });
    }

    Ok(next.run(req).await)
}
