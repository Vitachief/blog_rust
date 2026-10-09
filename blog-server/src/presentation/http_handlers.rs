use actix_web::{web, HttpResponse, Result};
use std::sync::Arc;
use crate::domain::{error::AppError, user::{RegisterRequest, LoginRequest}, post::{CreatePostRequest, UpdatePostRequest}};
use crate::application::{AuthService, BlogService};
use crate::presentation::middleware::AuthenticatedUser;

pub async fn register(
    payload: web::Json<RegisterRequest>,
    service: web::Data<Arc<AuthService>>,
) -> Result<HttpResponse, AppError> {
    let (token, user) = service.register(payload.into_inner()).await?;
    Ok(HttpResponse::Created().json(serde_json::json!({"token": token, "user": user})))
}

pub async fn login(
    payload: web::Json<LoginRequest>,
    service: web::Data<Arc<AuthService>>,
) -> Result<HttpResponse, AppError> {
    let (token, user) = service.login(payload.into_inner()).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"token": token, "user": user})))
}

pub async fn create_post(
    user: web::ReqData<AuthenticatedUser>,
    payload: web::Json<CreatePostRequest>,
    service: web::Data<Arc<BlogService>>,
) -> Result<HttpResponse, AppError> {
    let post = service.create_post(user.into_inner().user_id, payload.into_inner()).await?;
    Ok(HttpResponse::Created().json(post))
}

pub async fn get_post(
    path: web::Path<i64>,
    service: web::Data<Arc<BlogService>>,
) -> Result<HttpResponse, AppError> {
    let post = service.get_post(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(post))
}

pub async fn update_post(
    path: web::Path<i64>,
    user: web::ReqData<AuthenticatedUser>,
    payload: web::Json<UpdatePostRequest>,
    service: web::Data<Arc<BlogService>>,
) -> Result<HttpResponse, AppError> {
    let post = service.update_post(path.into_inner(), user.into_inner().user_id, payload.into_inner()).await?;
    Ok(HttpResponse::Ok().json(post))
}

pub async fn delete_post(
    path: web::Path<i64>,
    user: web::ReqData<AuthenticatedUser>,
    service: web::Data<Arc<BlogService>>,
) -> Result<HttpResponse, AppError> {
    service.delete_post(path.into_inner(), user.into_inner().user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn list_posts(
    query: web::Query<serde_json::Value>,
    service: web::Data<Arc<BlogService>>,
) -> Result<HttpResponse, AppError> {
    let limit = query.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as i32;
    let offset = query.get("offset").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    
    let (posts, total) = service.list_posts(limit, offset).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "posts": posts,
        "total": total,
        "limit": limit,
        "offset": offset
    })))
}