use actix_web::{dev::ServiceRequest, web, Error, HttpMessage};
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
    let jwt_service_data = req.app_data::<web::Data<Arc<JwtService>>>()
        .expect("JwtService не найден в app_data. Проверьте инициализацию в main.rs");

    match jwt_service_data.verify_token(credentials.token()) {
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