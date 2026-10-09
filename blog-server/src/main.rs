use actix_web::{web, App, HttpServer};
use actix_cors::Cors;
use actix_web_httpauth::middleware::HttpAuthentication;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

mod domain;
mod application;
mod data;
mod infrastructure;
mod presentation;

pub mod proto {
    tonic::include_proto!("blog");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let pool = infrastructure::database::create_pool(&database_url).await?;
    infrastructure::database::run_migrations(&pool).await?;

    let jwt_service = Arc::new(infrastructure::jwt::JwtService::new(&jwt_secret));
    let user_repo = Arc::new(data::user_repository::UserRepository::new(pool.clone()));
    let post_repo = Arc::new(data::post_repository::PostRepository::new(pool.clone()));
    
    let auth_service = Arc::new(application::auth_service::AuthService::new(user_repo, jwt_service.clone()));
    let blog_service = Arc::new(application::blog_service::BlogService::new(post_repo));

    // --- Клонирование ДЛЯ gRPC ---
    let auth_for_grpc = auth_service.clone();
    let blog_for_grpc = blog_service.clone();
    let jwt_for_grpc = jwt_service.clone();

    // --- HTTP Server ---
    let http_addr = std::env::var("HTTP_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let http_server = HttpServer::new(move || {
        let auth_middleware = HttpAuthentication::bearer(presentation::middleware::jwt_validator);
        
        App::new()
            .wrap(Cors::permissive())
            .app_data(web::Data::new(jwt_service.clone()))
            .app_data(web::Data::new(auth_service.clone()))
            .app_data(web::Data::new(blog_service.clone()))
            .route("/api/auth/register", web::post().to(presentation::http_handlers::register))
            .route("/api/auth/login", web::post().to(presentation::http_handlers::login))
            .route("/api/posts", web::get().to(presentation::http_handlers::list_posts))
            .route("/api/posts/{id}", web::get().to(presentation::http_handlers::get_post))
            .service(
                web::scope("/api/posts")
                    .wrap(auth_middleware)
                    .route("", web::post().to(presentation::http_handlers::create_post))
                    .route("/{id}", web::put().to(presentation::http_handlers::update_post))
                    .route("/{id}", web::delete().to(presentation::http_handlers::delete_post))
            )
    })
    .bind(&http_addr)?
    .run();

    // --- gRPC Server ---
    let grpc_addr_str = std::env::var("GRPC_ADDR").unwrap_or_else(|_| "127.0.0.1:50051".to_string());
    let grpc_addr = grpc_addr_str.parse().unwrap();
    let grpc_server = tonic::transport::Server::builder()
        .add_service(proto::blog_service_server::BlogServiceServer::new(
            presentation::grpc_service::BlogGrpcService::new(auth_for_grpc, blog_for_grpc, jwt_for_grpc),
        ))
        .serve(grpc_addr);

    tracing::info!("HTTP сервер запущен на {}", http_addr);
    tracing::info!("gRPC сервер запущен на {}", grpc_addr_str);

    // Запуск gRPC сервера в фоновой задаче Tokio
    tokio::spawn(grpc_server);
    
    // HTTP сервер блокирует основной поток до завершения
    http_server.await?;
    
    Ok(())
}