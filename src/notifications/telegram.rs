use std::time::Duration;

use reqwest::{multipart, Client};
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
    let url = format!("https://api.telegram.org/bot{token}/sendPhoto");
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| NotificationError::TelegramFailed(e.to_string()))?;
    let filename = path
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| "snapshot.jpg".to_string());
    let caption = format!("<b>{}</b>\n{}", escape_html(title), escape_html(body));
    let mut form = multipart::Form::new()
        .text("chat_id", chat_id.to_string())
        .text("caption", caption)
        .text("parse_mode", "HTML")
        .part("photo", multipart::Part::bytes(bytes).file_name(filename));

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
