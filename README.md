# Blog Project

Полноценная система блога на Rust, реализованная с соблюдением принципов **Clean Architecture**. Проект состоит из четырёх крейтов в едином Cargo workspace:

- **`blog-server`** — бэкенд с HTTP (Actix-web) и gRPC (Tonic) API, работающий с PostgreSQL через SQLx.
- **`blog-client`** — клиентская библиотека с единым интерфейсом для HTTP и gRPC транспортов.
- **`blog-cli`** — консольный клиент для взаимодействия с сервером.
- **`blog-wasm`** — фронтенд, компилируемый в WebAssembly.

---

## Архитектура сервера

Сервер построен по принципам Clean Architecture с чётким разделением ответственности:

```
blog-server/src/
├── domain/           # Доменные модели (User, Post) и ошибки
├── application/      # Бизнес-логика (AuthService, BlogService)
├── data/             # Репозитории для работы с БД
├── infrastructure/   # БД, JWT, Argon2, логирование
└── presentation/     # HTTP handlers, gRPC сервис, middleware
```

**Ключевые особенности:**
- HTTP API на порту `8080`, gRPC API на порту `50051`
- Оба сервера запускаются **параллельно** в одном Tokio runtime
- Общая бизнес-логика (`AuthService`, `BlogService`) используется и HTTP, и gRPC handlers — **никакого дублирования кода**
- Пароли хешируются алгоритмом **Argon2** с автоматической генерацией соли
- Аутентификация через **JWT-токены** (срок жизни 24 часа)
- Логирование через **tracing** с фильтрацией по `RUST_LOG`
- CORS настроен для работы с WASM-фронтендом

---

## Требования

- **Rust** 1.85+ (рекомендуется 1.99+)
- **PostgreSQL** 12+
- **protobuf-compiler** (`protoc`)
- **sqlx-cli** (опционально, для ручного управления миграциями)

### Установка зависимостей (Ubuntu/WSL)

```bash
# Rust (если ещё не установлен)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# PostgreSQL
sudo apt update
sudo apt install postgresql postgresql-contrib protobuf-compiler

# sqlx-cli (с rustls, чтобы не тянуть OpenSSL)
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

---

## Настройка окружения

### 1. Создание базы данных

```bash
# Запускаем PostgreSQL
sudo service postgresql start

# Создаём пользователя и базу данных
sudo -u postgres psql -c "CREATE USER blog_user WITH PASSWORD 'blog_password';"
sudo -u postgres psql -c "CREATE DATABASE blog_db OWNER blog_user;"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE blog_db TO blog_user;"
```

### 2. Файл `.env`

Создайте файл **`blog-server/.env`** со следующим содержимым:

```env
# Подключение к PostgreSQL
DATABASE_URL=postgres://blog_user:blog_password@localhost/blog_db

# Секретный ключ для подписи JWT-токенов (минимум 32 символа)
JWT_SECRET=super_secret_key_at_least_32_chars_long_for_security_123

# Настройки адресов серверов
HTTP_ADDR=127.0.0.1:8080
GRPC_ADDR=127.0.0.1:50051

# Уровень логирования: error, warn, info, debug, trace
RUST_LOG=info
```

> ⚠️ **Важно:**
> - Файл `.env` **не коммитится** в git (уже добавлен в `.gitignore`).
> - `JWT_SECRET` должен быть **уникальным** для каждого развёртывания. Никогда не используйте один и тот же секрет в dev и prod.
> - Сервер слушает **только `127.0.0.1`** (localhost) — это безопаснее, чем `0.0.0.0`, так как блокирует внешние подключения.

### 3. Применение миграций

Миграции применяются **автоматически** при старте сервера через макрос `sqlx::migrate!()`. Но их можно применить и вручную:

```bash
cd blog-server
sqlx migrate run
sqlx migrate info   # показать статус миграций
```

Миграции идемпотентны (используют `IF NOT EXISTS`), поэтому их можно запускать повторно без ошибок.

---

## Запуск сервера

```bash
cd blog-server
cargo run
```

При успешном запуске вы увидите:

```
INFO Миграции успешно применены
INFO HTTP сервер запущен на 127.0.0.1:8080
INFO gRPC сервер запущен на 127.0.0.1:50051
```

Сервер будет работать, пока вы не прервёте его (`Ctrl+C`).

---

## HTTP API

### Публичные endpoints (без авторизации)

| Метод | Путь                | Описание                          |
|-------|---------------------|-----------------------------------|
| POST  | `/api/auth/register`| Регистрация нового пользователя   |
| POST  | `/api/auth/login`   | Вход, получение JWT-токена        |
| GET   | `/api/posts`        | Список постов (пагинация)         |
| GET   | `/api/posts/{id}`   | Получение поста по ID             |

### Защищённые endpoints (требуют `Authorization: Bearer <token>`)

| Метод  | Путь                | Описание                          |
|--------|---------------------|-----------------------------------|
| POST   | `/api/posts`        | Создание поста                    |
| PUT    | `/api/posts/{id}`   | Обновление поста (только автор)   |
| DELETE | `/api/posts/{id}`   | Удаление поста (только автор)     |

### Примеры запросов через `curl`

**Регистрация:**
```bash
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "email": "test@example.com", "password": "supersecret"}'
```

**Вход:**
```bash
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "password": "supersecret"}'
```

**Создание поста** (замените `YOUR_TOKEN` на токен из ответа login/register):
```bash
curl -X POST http://localhost:8080/api/posts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{"title": "Мой первый пост", "content": "Привет, мир!"}'
```

**Список постов:**
```bash
curl "http://localhost:8080/api/posts?limit=10&offset=0"
```

---

## gRPC API

gRPC-сервис описан в `blog-server/proto/blog.proto` и предоставляет те же методы:
- `Register`, `Login` — аутентификация
- `CreatePost`, `GetPost`, `UpdatePost`, `DeletePost`, `ListPosts` — CRUD постов

JWT-токен передаётся через **метаданные** запроса в заголовке `authorization: Bearer <token>`.

---

## Безопасность

- **Пароли** хешируются алгоритмом **Argon2id** с автоматической генерацией уникальной соли для каждого пользователя. Соль хранится вместе с хэшем в формате PHC String Format.
- **JWT-токены** подписываются алгоритмом HS256, срок жизни — 24 часа. Claims содержат `user_id` и `username`.
- **SQL-инъекции** исключены: все запросы параметризованы через `sqlx`.
- **CORS** настроен на `permissive`

---

## Структура проекта

```
blog-project/
├── Cargo.toml              # Workspace конфигурация
├── README.md               # Этот файл
├── .gitignore
└── blog-server/
    ├── Cargo.toml
    ├── build.rs            # Генерация Rust-кода из .proto
    ├── .env                # Переменные окружения (НЕ коммитится)
    ├── proto/
    │   └── blog.proto      # gRPC-схема
    ├── migrations/         # SQL-миграции (создаются через sqlx-cli)
    └── src/
        ├── main.rs
        ├── domain/
        ├── application/
        ├── data/
        ├── infrastructure/
        └── presentation/
```

### Сервер не стартует на порту 8080
Проверьте, не занят ли порт: `lsof -i :8080`. Или измените `HTTP_ADDR` в `.env`.

---

## Клиентская библиотека (`blog-client`)

Единая клиентская библиотека для взаимодействия с API блога. Её главная цель — **абстрагировать транспортный уровень**, позволяя приложениям (например, CLI или другим микросервисам) переключаться между HTTP и gRPC без изменения бизнес-логики.

### Ключевые особенности
- **Единый интерфейс**: Структура `BlogClient` предоставляет одинаковые методы (`login`, `create_post`, `list_posts` и т.д.) независимо от выбранного транспорта.
- **Автоматическое управление токеном**: После вызова `login` или `register` JWT-токен сохраняется внутри экземпляра клиента и автоматически подставляется в заголовок `Authorization` (для HTTP) или в метаданные (для gRPC) при защищённых запросах.
- **Безопасный транспорт**: HTTP-клиент использует `reqwest` с движком `rustls` (чистый Rust, без зависимостей от системного OpenSSL).
- **Строгая типизация**: Все ответы десериализуются в строго типизированные структуры (`AuthResponse`, `Post`, `ListPostsResponse`).
- **Централизованная обработка ошибок**: Кастомный enum `BlogClientError` с понятными вариантами (`Unauthorized`, `NotFound`, `AuthRequired` и т.д.).

### Пример использования

```rust
use blog_client::{BlogClient, Transport};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Инициализация клиента с выбором транспорта (HTTP или gRPC)
    let mut client = BlogClient::new(Transport::Http("http://127.0.0.1:8080".to_string())).await?;
    // Для gRPC: Transport::Grpc("http://127.0.0.1:50051".to_string())

    // 2. Аутентификация (токен автоматически сохранится внутри клиента)
    let auth_resp = client.login("testuser".into(), "supersecret".into()).await?;
    println!("Вход выполнен! Привет, {}", auth_resp.user.username);

    // 3. Создание поста (метод сам проверит наличие токена и добавит его в запрос)
    let post = client.create_post("Мой первый пост".into(), "Содержание поста".into()).await?;
    println!("Создан пост с ID: {}", post.id);

    // 4. Получение списка постов (публичный endpoint, токен не обязателен)
    let list = client.list_posts(10, 0).await?;
    println!("Всего постов в базе: {}", list.total);

    Ok(())
}
```

### Как это работает под капотом
- При инициализации `BlogClient` создаёт либо `reqwest::Client`, либо `tonic::BlogServiceClient`.
- Для gRPC-запросов используется паттерн клонирования клиента (`self.client.clone()`), что является стандартной и дешёвой практикой в `tonic` (клонируется внутренний `Arc` канала, а не создаётся новое сетевое соединение). Это позволяет сохранить удобный API с неизменяемыми ссылками `&self`.
- Файл `build.rs` автоматически генерирует Rust-код из единого файла `../blog-server/proto/blog.proto`, гарантируя, что клиент и сервер всегда используют одинаковые контракты.

### Сборка библиотеки
```bash
cargo build -p blog-client
```
*(Убедитесь, что в системе установлен `protobuf-compiler`, так как он необходим для генерации кода из `.proto` файла).*
