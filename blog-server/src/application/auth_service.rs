use std::sync::Arc;
use crate::domain::{error::AppError, user::{RegisterRequest, LoginRequest, User}};
use crate::data::user_repository::UserRepository;
use crate::infrastructure::{auth::verify_password, jwt::JwtService};

pub struct AuthService {
    user_repo: Arc<UserRepository>,
    jwt_service: Arc<JwtService>,
}

impl AuthService {
    pub fn new(user_repo: Arc<UserRepository>, jwt_service: Arc<JwtService>) -> Self {
        Self { user_repo, jwt_service }
    }

    pub async fn register(&self, req: RegisterRequest) -> Result<(String, User), AppError> {
        let user = self.user_repo.create(&req).await?;
        let token = self.jwt_service.generate_token(user.id, &user.username)?;
        Ok((token, user))
    }

    pub async fn login(&self, req: LoginRequest) -> Result<(String, User), AppError> {
        let user_opt = self.user_repo.get_by_username(&req.username).await?;
        let user = user_opt.ok_or(AppError::InvalidCredentials)?;
            
        if !verify_password(&req.password, &user.password_hash)? {
            return Err(AppError::InvalidCredentials);
        }
        
        let token = self.jwt_service.generate_token(user.id, &user.username)?;
        Ok((token, user))
    }
}