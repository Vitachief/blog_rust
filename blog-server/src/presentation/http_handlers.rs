use actix_web::{web, HttpResponse, Result};
use std::sync::Arc;
use crate::domain::{error::AppError, user::{RegisterRequest, LoginRequest}, post::{CreatePostRequest, UpdatePostRequest}};
use crate::application::{auth_service::AuthService, blog_service::BlogService};
use crate::presentation::middleware::AuthenticatedUser;

pub async fn register(
    payload: web::Json<RegisterRequest>,
    service: web::Data<Arc<AuthService>>,
) -> Result<HttpResponse, AppError> {
    // Извлекаем данные ДО того, как payload будет перемещён (moved)
    let req = payload.into_inner();
    let username = req.username.clone();
    let email = req.email.clone();

    tracing::info!("Запрос регистрации пользователя: username='{}', email='{}'", username, email);
    
    match service.register(req).await {
        Ok((token, user)) => {
            tracing::info!("Пользователь успешно зарегистрирован: id={}, username='{}'", 
                user.id, user.username);
            Ok(HttpResponse::Created().json(serde_json::json!({"token": token, "user": user})))
        }
        Err(e) => {
            tracing::error!("Ошибка регистрации: {}", e);
            Err(e)
        }
    }
}

pub async fn login(
    payload: web::Json<LoginRequest>,
    service: web::Data<Arc<AuthService>>,
) -> Result<HttpResponse, AppError> {
    // Извлекаем данные ДО перемещения
    let req = payload.into_inner();
    let username = req.username.clone();

    tracing::info!("Запрос входа пользователя: username='{}'", username);
    
    match service.login(req).await {
        Ok((token, user)) => {
            tracing::info!("Пользователь успешно вошёл: id={}, username='{}'", 
                user.id, user.username);
            Ok(HttpResponse::Ok().json(serde_json::json!({"token": token, "user": user})))
        }
        Err(e) => {
            tracing::warn!("Неудачная попытка входа: username='{}', ошибка={}", username, e);
            Err(e)
        }
    }
}

pub async fn create_post(
    user: web::ReqData<AuthenticatedUser>,
    payload: web::Json<CreatePostRequest>,
    service: web::Data<Arc<BlogService>>,
) -> Result<HttpResponse, AppError> {
    let user_data = user.into_inner();
    let post_data = payload.into_inner();
    let title = post_data.title.clone(); // Клонируем для логирования
    
    tracing::info!("Запрос создания поста: user_id={}, username='{}', title='{}'", 
        user_data.user_id, user_data._username, title);
    
    match service.create_post(user_data.user_id, post_data).await {
        Ok(post) => {
            tracing::info!("Пост успешно создан: id={}, title='{}', author_id={}", 
                post.id, post.title, post.author_id);
            Ok(HttpResponse::Created().json(post))
        }
        Err(e) => {
            tracing::error!("Ошибка создания поста: user_id={}, ошибка={}", 
                user_data.user_id, e);
            Err(e)
        }
    }
}

pub async fn get_post(
    path: web::Path<i64>,
    service: web::Data<Arc<BlogService>>,
) -> Result<HttpResponse, AppError> {
    let post_id = path.into_inner();
    tracing::info!("Запрос получения поста: id={}", post_id);
    
    match service.get_post(post_id).await {
        Ok(post) => {
            tracing::info!("Пост успешно получен: id={}, title='{}'", post_id, post.title);
            Ok(HttpResponse::Ok().json(post))
        }
        Err(e) => {
            tracing::warn!("Пост не найден или ошибка: id={}, ошибка={}", post_id, e);
            Err(e)
        }
    }
}

pub async fn update_post(
    path: web::Path<i64>,
    user: web::ReqData<AuthenticatedUser>,
    payload: web::Json<UpdatePostRequest>,
    service: web::Data<Arc<BlogService>>,
) -> Result<HttpResponse, AppError> {
    let post_id = path.into_inner();
    let user_data = user.into_inner();
    let post_data = payload.into_inner();
    
    tracing::info!("Запрос обновления поста: post_id={}, user_id={}, username='{}'", 
        post_id, user_data.user_id, user_data._username);
    
    match service.update_post(post_id, user_data.user_id, post_data).await {
        Ok(post) => {
            tracing::info!("Пост успешно обновлён: id={}, title='{}'", post.id, post.title);
            Ok(HttpResponse::Ok().json(post))
        }
        Err(e) => {
            tracing::error!("Ошибка обновления поста: post_id={}, user_id={}, ошибка={}", 
                post_id, user_data.user_id, e);
            Err(e)
        }
    }
}

pub async fn delete_post(
    path: web::Path<i64>,
    user: web::ReqData<AuthenticatedUser>,
    service: web::Data<Arc<BlogService>>,
) -> Result<HttpResponse, AppError> {
    let post_id = path.into_inner();
    let user_data = user.into_inner();
    
    tracing::info!("Запрос удаления поста: post_id={}, user_id={}, username='{}'", 
        post_id, user_data.user_id, user_data._username);
    
    match service.delete_post(post_id, user_data.user_id).await {
        Ok(()) => {
            tracing::info!("Пост успешно удалён: post_id={}, user_id={}", 
                post_id, user_data.user_id);
            Ok(HttpResponse::NoContent().finish())
        }
        Err(e) => {
            tracing::error!("Ошибка удаления поста: post_id={}, user_id={}, ошибка={}", 
                post_id, user_data.user_id, e);
            Err(e)
        }
    }
}

pub async fn list_posts(
    query: web::Query<serde_json::Value>,
    service: web::Data<Arc<BlogService>>,
) -> Result<HttpResponse, AppError> {
    let limit = query.get("limit").and_then(|v| v.as_i64()).unwrap_or(10) as i32;
    let offset = query.get("offset").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    
    tracing::info!("Запрос списка постов: limit={}, offset={}", limit, offset);
    
    match service.list_posts(limit, offset).await {
        Ok((posts, total)) => {
            tracing::info!("Список постов получен: total={}, returned={}", total, posts.len());
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "posts": posts,
                "total": total,
                "limit": limit,
                "offset": offset
            })))
        }
        Err(e) => {
            tracing::error!("Ошибка получения списка постов: {}", e);
            Err(e)
        }
    }
}