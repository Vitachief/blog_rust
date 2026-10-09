use anyhow::Result;
use blog_client::{BlogClient, Transport};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;

/// CLI-клиент для блога. Поддерживает HTTP и gRPC транспорты.
#[derive(Parser)]
#[command(
    name = "blog-cli",
    about = "Консольный клиент для управления блогом",
    version
)]
struct Cli {
    /// Адрес HTTP-сервера
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    server: String,

    /// Использовать gRPC вместо HTTP
    #[arg(long)]
    grpc: bool,

    /// Адрес gRPC-сервера (используется только с флагом --grpc)
    #[arg(long, default_value = "http://127.0.0.1:50051")]
    grpc_server: String,

    /// Путь к файлу для хранения JWT-токена
    #[arg(long, default_value = ".blog_token")]
    token_file: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Регистрация нового пользователя
    Register {
        #[arg(long)]
        username: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        password: String,
    },
    /// Вход в систему
    Login {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
    },
    /// Создание нового поста
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        content: String,
    },
    /// Получение поста по ID
    Get {
        #[arg(long)]
        id: i64,
    },
    /// Обновление поста (title и/или content)
    Update {
        #[arg(long)]
        id: i64,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        content: Option<String>,
    },
    /// Удаление поста по ID
    Delete {
        #[arg(long)]
        id: i64,
    },
    /// Список постов (с пагинацией)
    List {
        #[arg(long, default_value = "10")]
        limit: i32,
        #[arg(long, default_value = "0")]
        offset: i32,
    },
}

/// Загружает токен из файла, если он существует
fn load_token(path: &str) -> Option<String> {
    if Path::new(path).exists() {
        fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    }
}

/// Сохраняет токен в файл
fn save_token(path: &str, token: &str) -> Result<()> {
    fs::write(path, token)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 1. Определяем транспорт
    let transport = if cli.grpc {
        println!("[INFO] Используем gRPC: {}", cli.grpc_server);
        Transport::Grpc(cli.grpc_server.clone())
    } else {
        println!("[INFO] Используем HTTP: {}", cli.server);
        Transport::Http(cli.server.clone())
    };

    // 2. Создаём клиент
    let mut client = BlogClient::new(transport).await?;

    // 3. Загружаем сохранённый токен (если есть)
    if let Some(token) = load_token(&cli.token_file) {
        client.set_token(token);
    }

    // 4. Выполняем команду
    match cli.command {
        Commands::Register { username, email, password } => {
            println!("[INFO] Регистрация пользователя '{}'...", username);
            let resp = client.register(username.clone(), email, password).await?;
            save_token(&cli.token_file, &resp.token)?;
            println!("[OK] Регистрация успешна!");
            println!(
                "[OK] Пользователь: id={}, username={}",
                resp.user.id, resp.user.username
            );
            println!("[OK] Токен сохранён в {}", cli.token_file);
        }

        Commands::Login { username, password } => {
            println!("[INFO] Вход как '{}'...", username);
            let resp = client.login(username.clone(), password).await?;
            save_token(&cli.token_file, &resp.token)?;
            println!("[OK] Вход выполнен!");
            println!(
                "[OK] Пользователь: id={}, username={}",
                resp.user.id, resp.user.username
            );
            println!("[OK] Токен сохранён в {}", cli.token_file);
        }

        Commands::Create { title, content } => {
            println!("[INFO] Создание поста '{}'...", title);
            let post = client.create_post(title, content).await?;
            println!("[OK] Пост создан!");
            println!("{}", serde_json::to_string_pretty(&post)?);
        }

        Commands::Get { id } => {
            println!("[INFO] Получение поста #{}...", id);
            let post = client.get_post(id).await?;
            println!("{}", serde_json::to_string_pretty(&post)?);
        }

        Commands::Update { id, title, content } => {
            println!("[INFO] Обновление поста #{}...", id);
            let post = client.update_post(id, title, content).await?;
            println!("[OK] Пост обновлён!");
            println!("{}", serde_json::to_string_pretty(&post)?);
        }

        Commands::Delete { id } => {
            println!("[INFO] Удаление поста #{}...", id);
            client.delete_post(id).await?;
            println!("[OK] Пост удалён!");
        }

        Commands::List { limit, offset } => {
            println!(
                "[INFO] Список постов (limit={}, offset={})...",
                limit, offset
            );
            let resp = client.list_posts(limit, offset).await?;
            println!("[INFO] Всего постов: {}", resp.total);
            println!("[INFO] Показано: {}\n", resp.posts.len());

            // Сохраняем длину ДО цикла, так как цикл забирает владение вектором
            let posts_count = resp.posts.len();

            for post in resp.posts {
                println!("----------------------------------------");
                println!("#{} | {}", post.id, post.title);
                println!("Автор: {} | {}", post.author_id, post.created_at);
                println!("{}", post.content);
            }
            if posts_count > 0 {
                println!("----------------------------------------");
            }
        }
    }

    Ok(())
}