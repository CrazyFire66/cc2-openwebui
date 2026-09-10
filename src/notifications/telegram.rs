use std::time::Duration;

use reqwest::{multipart, Client};
use serde::Deserialize;
use serde_json::json;
use tracing::warn;

use crate::config::NotificationDestination;
use crate::error::NotificationError;

pub async fn send_test(dest: &NotificationDestination) -> Result<(), NotificationError> {
    send(dest, "CC2 Monitor", "Test notification - Telegram bot is working", None).await
}

pub async fn send(
    dest: &NotificationDestination,
    title: &str,
    body: &str,
    snapshot_path: Option<&std::path::Path>,
) -> Result<(), NotificationError> {
    let token = dest
        .telegram_bot_token
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| NotificationError::TelegramFailed("bot token is not configured".to_string()))?;
    let chat_id = dest
        .telegram_chat_id
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| NotificationError::TelegramFailed("chat id is not configured".to_string()))?;

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_default();

    if let Some(path) = snapshot_path {
        if path.is_file() {
            return send_photo(&client, token, chat_id, dest.telegram_thread_id.as_deref(), title, body, path).await;
        }
    }

    send_message(&client, token, chat_id, dest.telegram_thread_id.as_deref(), title, body).await
}

pub async fn send_status(
    dest: &NotificationDestination,
    title: &str,
    body: &str,
) -> Result<(), NotificationError> {
    send(dest, title, body, None).await
}

pub async fn send_status_photo(
    dest: &NotificationDestination,
    title: &str,
    body: &str,
    bytes: Vec<u8>,
) -> Result<(), NotificationError> {
    let token = dest
        .telegram_bot_token
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| NotificationError::TelegramFailed("bot token is not configured".to_string()))?;
    let chat_id = dest
        .telegram_chat_id
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| NotificationError::TelegramFailed("chat id is not configured".to_string()))?;
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap_or_default();

    send_photo_bytes(
        &client,
        token,
        chat_id,
        dest.telegram_thread_id.as_deref(),
        title,
        body,
        bytes,
        "status.jpg",
    ).await
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    #[serde(default)]
    pub message: Option<TelegramMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramMessage {
    #[serde(default)]
    pub message_thread_id: Option<i64>,
    #[serde(default)]
    pub text: Option<String>,
    pub chat: TelegramChat,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
struct TelegramApiResponse<T> {
    ok: bool,
    #[serde(default)]
    result: Option<T>,
    #[serde(default)]
    description: Option<String>,
}

pub async fn get_updates(
    dest: &NotificationDestination,
    offset: Option<i64>,
    timeout_secs: u8,
) -> Result<Vec<TelegramUpdate>, NotificationError> {
    let token = dest
        .telegram_bot_token
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| NotificationError::TelegramFailed("bot token is not configured".to_string()))?;
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs as u64 + 10))
        .build()
        .unwrap_or_default();
    let url = format!("https://api.telegram.org/bot{token}/getUpdates");
    let mut payload = json!({
        "timeout": timeout_secs,
        "allowed_updates": ["message"],
    });
    if let Some(offset) = offset {
        payload["offset"] = json!(offset);
    }
    let res = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| NotificationError::TelegramFailed(e.to_string()))?;
    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        warn!("telegram getUpdates returned {status}: {text}");
        return Err(NotificationError::TelegramFailed(format!("server returned {status}")));
    }
    let body = res
        .json::<TelegramApiResponse<Vec<TelegramUpdate>>>()
        .await
        .map_err(|e| NotificationError::TelegramFailed(e.to_string()))?;
    if !body.ok {
        return Err(NotificationError::TelegramFailed(
            body.description.unwrap_or_else(|| "getUpdates failed".to_string()),
        ));
    }
    Ok(body.result.unwrap_or_default())
}

async fn send_message(
    client: &Client,
    token: &str,
    chat_id: &str,
    thread_id: Option<&str>,
    title: &str,
    body: &str,
) -> Result<(), NotificationError> {
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let mut payload = json!({
        "chat_id": chat_id,
        "text": format!("<b>{}</b>\n{}", escape_html(title), escape_html(body)),
        "parse_mode": "HTML",
        "disable_web_page_preview": true,
    });
    add_thread_id(&mut payload, thread_id);

    let res = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| NotificationError::TelegramFailed(e.to_string()))?;

    ensure_success(res, "sendMessage").await
}

async fn send_photo(
    client: &Client,
    token: &str,
    chat_id: &str,
    thread_id: Option<&str>,
    title: &str,
    body: &str,
    path: &std::path::Path,
) -> Result<(), NotificationError> {
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| NotificationError::TelegramFailed(e.to_string()))?;
    let filename = path
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| "snapshot.jpg".to_string());
    send_photo_bytes(client, token, chat_id, thread_id, title, body, bytes, &filename).await
}

async fn send_photo_bytes(
    client: &Client,
    token: &str,
    chat_id: &str,
    thread_id: Option<&str>,
    title: &str,
    body: &str,
    bytes: Vec<u8>,
    filename: &str,
) -> Result<(), NotificationError> {
    let url = format!("https://api.telegram.org/bot{token}/sendPhoto");
    let caption = format!("<b>{}</b>\n{}", escape_html(title), escape_html(body));
    let mut form = multipart::Form::new()
        .text("chat_id", chat_id.to_string())
        .text("caption", caption)
        .text("parse_mode", "HTML")
        .part("photo", multipart::Part::bytes(bytes).file_name(filename.to_string()));

    if let Some(id) = thread_id.and_then(|v| v.trim().parse::<i64>().ok()) {
        form = form.text("message_thread_id", id.to_string());
    }

    let res = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| NotificationError::TelegramFailed(e.to_string()))?;

    ensure_success(res, "sendPhoto").await
}

fn add_thread_id(payload: &mut serde_json::Value, thread_id: Option<&str>) {
    let Some(id) = thread_id.and_then(|v| v.trim().parse::<i64>().ok()) else {
        return;
    };
    payload["message_thread_id"] = json!(id);
}

async fn ensure_success(res: reqwest::Response, method: &str) -> Result<(), NotificationError> {
    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        warn!("telegram {method} returned {status}: {text}");
        return Err(NotificationError::TelegramFailed(format!("server returned {status}")));
    }
    Ok(())
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
