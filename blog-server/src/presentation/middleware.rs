use actix_web::{dev::ServiceRequest, Error, HttpMessage};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use std::sync::Arc;
use crate::infrastructure::jwt::JwtService;

#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub user_id: i64,
    pub _username: String,
}

pub async fn jwt_validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let jwt_service = req.app_data::<Arc<JwtService>>().unwrap();
    match jwt_service.verify_token(credentials.token()) {
        Ok(claims) => {
            req.extensions_mut().insert(AuthenticatedUser {
                user_id: claims.user_id,
                _username: claims.username,
            });
            Ok(req)
        }
        Err(_) => Err((actix_web::error::ErrorUnauthorized("Invalid or expired token"), req)),
    }
}