use wasm_bindgen::prelude::*;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use web_sys::console;

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub author_name: String,
    pub created_at: String,
}

fn get_token() -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item("blog_token").ok().flatten())
}

fn set_token(token: &str) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.set_item("blog_token", token);
    }
}

fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    value
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Преобразует HTTP-ответ с ошибкой в понятное сообщение для пользователя.
/// Пытается извлечь текст ошибки из тела ответа, иначе использует HTTP-код.
async fn handle_api_error(response: gloo_net::http::Response) -> JsValue {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    let user_message = match status {
        400 => "Неверный запрос",
        401 => "Требуется авторизация",
        403 => {
            if body.contains("not the author") || body.contains("Forbidden") {
                "У вас нет прав для этой операции (вы не являетесь автором)"
            } else {
                "Доступ запрещён"
            }
        }
        404 => "Объект не найден",
        409 => "Конфликт данных (возможно, пользователь с таким именем уже существует)",
        500 => "Внутренняя ошибка сервера",
        _ => "Неизвестная ошибка",
    };

    // Если сервер вернул осмысленный текст — используем его, иначе шаблонное сообщение
    let final_message = if !body.is_empty() && body.len() < 200 {
        format!("{}: {}", user_message, body)
    } else {
        format!("{} (HTTP {})", user_message, status)
    };

    JsValue::from_str(&final_message)
}

#[wasm_bindgen]
pub struct BlogApp {
    server_url: String,
}

#[wasm_bindgen]
impl BlogApp {
    #[wasm_bindgen(constructor)]
    pub fn new(server_url: String) -> Self {
        console::log_1(&"[INFO] BlogApp initialized".into());
        Self { server_url }
    }

    #[wasm_bindgen]
    pub fn clear_token(&self) {
        if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = ls.remove_item("blog_token");
        }
        console::log_1(&"[INFO] Token cleared".into());
    }

    #[wasm_bindgen]
    pub fn is_authenticated(&self) -> bool {
        get_token().is_some()
    }

    #[wasm_bindgen]
    pub async fn register(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<JsValue, JsValue> {
        let payload = serde_json::json!({
            "username": username,
            "email": email,
            "password": password
        });

        let response = Request::post(&format!("{}/api/auth/register", self.server_url))
            .json(&payload)
            .map_err(|e| JsValue::from_str(&format!("Ошибка формирования запроса: {}", e)))?
            .send()
            .await
            .map_err(|e| JsValue::from_str(&format!("Ошибка сети: {}", e)))?;

        if !response.ok() {
            return Err(handle_api_error(response).await);
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        if let Some(token) = json["token"].as_str() {
            set_token(token);
        }

        to_js_value(&json)
    }

    #[wasm_bindgen]
    pub async fn login(&self, username: String, password: String) -> Result<JsValue, JsValue> {
        let payload = serde_json::json!({
            "username": username,
            "password": password
        });

        let response = Request::post(&format!("{}/api/auth/login", self.server_url))
            .json(&payload)
            .map_err(|e| JsValue::from_str(&e.to_string()))?
            .send()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        if !response.ok() {
            let status = response.status();
            if status == 401 {
                return Err(JsValue::from_str("Неверное имя пользователя или пароль"));
            }
            return Err(handle_api_error(response).await);
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        if let Some(token) = json["token"].as_str() {
            set_token(token);
        }

        to_js_value(&json)
    }

    #[wasm_bindgen]
    pub async fn load_posts(&self) -> Result<JsValue, JsValue> {
        let response = Request::get(&format!("{}/api/posts", self.server_url))
            .send()
            .await
            .map_err(|e| JsValue::from_str(&format!("Ошибка сети: {}", e)))?;

        if !response.ok() {
            return Err(handle_api_error(response).await);
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let posts: Vec<Post> = serde_json::from_value(json["posts"].clone())
            .map_err(|e| JsValue::from_str(&format!("Ошибка парсинга: {}", e)))?;

        to_js_value(&posts)
    }

    #[wasm_bindgen]
    pub async fn create_post(&self, title: String, content: String) -> Result<JsValue, JsValue> {
        let token = get_token().ok_or_else(|| JsValue::from_str("Требуется авторизация. Пожалуйста, войдите в систему."))?;
        let payload = serde_json::json!({
            "title": title,
            "content": content
        });

        let response = Request::post(&format!("{}/api/posts", self.server_url))
            .header("Authorization", &format!("Bearer {}", token))
            .json(&payload)
            .map_err(|e| JsValue::from_str(&e.to_string()))?
            .send()
            .await
            .map_err(|e| JsValue::from_str(&format!("Ошибка сети: {}", e)))?;

        if !response.ok() {
            return Err(handle_api_error(response).await);
        }

        let post: Post = response
            .json()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        to_js_value(&post)
    }

    #[wasm_bindgen]
    pub async fn delete_post(&self, id: f64) -> Result<JsValue, JsValue> {
        let token = get_token().ok_or_else(|| JsValue::from_str("Требуется авторизация. Пожалуйста, войдите в систему."))?;
        let id = id as i64;

        let response = Request::delete(&format!("{}/api/posts/{}", self.server_url, id))
            .header("Authorization", &format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| JsValue::from_str(&format!("Ошибка сети: {}", e)))?;

        if !response.ok() {
            return Err(handle_api_error(response).await);
        }

        Ok(JsValue::from_str("Пост успешно удалён"))
    }

    #[wasm_bindgen]
    pub async fn update_post(
        &self,
        id: f64,
        title: String,
        content: String,
    ) -> Result<JsValue, JsValue> {
        let token = get_token().ok_or_else(|| JsValue::from_str("Требуется авторизация. Пожалуйста, войдите в систему."))?;
        let id = id as i64;
        let payload = serde_json::json!({
            "title": title,
            "content": content
        });

        let response = Request::put(&format!("{}/api/posts/{}", self.server_url, id))
            .header("Authorization", &format!("Bearer {}", token))
            .json(&payload)
            .map_err(|e| JsValue::from_str(&e.to_string()))?
            .send()
            .await
            .map_err(|e| JsValue::from_str(&format!("Ошибка сети: {}", e)))?;

        if !response.ok() {
            return Err(handle_api_error(response).await);
        }

        let post: Post = response
            .json()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        to_js_value(&post)
    }
}