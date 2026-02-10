use crate::types::{Attachment, InboundMessage};
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use tokio::time::sleep;

pub async fn start_telegram_poller(
    token: String,
    tx: tokio::sync::mpsc::Sender<InboundMessage>,
    interval_seconds: u64,
) {
    let client = Client::new();
    let mut offset: i64 = 0;
    loop {
        let url = format!("https://api.telegram.org/bot{}/getUpdates", token);
        let resp = client
            .get(&url)
            .query(&[("timeout", "30"), ("offset", &offset.to_string())])
            .send()
            .await;
        if let Ok(resp) = resp {
            if let Ok(value) = resp.json::<Value>().await {
                if value.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                    if let Some(results) = value.get("result").and_then(|v| v.as_array()) {
                        for update in results {
                            if let Some(update_id) = update.get("update_id").and_then(|v| v.as_i64())
                            {
                                offset = update_id + 1;
                            }
                            if let Some(msg) = parse_telegram_update(update) {
                                let _ = tx.send(msg).await;
                            }
                        }
                    }
                }
            }
        }
        sleep(std::time::Duration::from_secs(interval_seconds)).await;
    }
}

pub fn parse_telegram_update(update: &Value) -> Option<InboundMessage> {
    let msg = update.get("message").or_else(|| update.get("channel_post"))?;
    let chat = msg.get("chat")?;
    let chat_id = chat.get("id")?.as_i64()?.to_string();
    let msg_id = msg.get("message_id")?.as_i64()?.to_string();
    let text = msg.get("text").and_then(|v| v.as_str()).map(|s| s.to_string());
    let thread_id = msg
        .get("message_thread_id")
        .and_then(|v| v.as_i64())
        .map(|v| v.to_string());

    let peer_kind = match chat.get("type").and_then(|v| v.as_str()) {
        Some("private") => "dm",
        Some("channel") => "channel",
        _ => "group",
    };

    let mut attachments = Vec::new();
    if let Some(photo) = msg
        .get("photo")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.last())
    {
        if let Some(file_id) = photo.get("file_id").and_then(|v| v.as_str()) {
            attachments.push(Attachment {
                id: Some(file_id.to_string()),
                url: format!("telegram://file/{}", file_id),
                mime_type: Some("image/jpeg".to_string()),
                filename: None,
                size: photo.get("file_size").and_then(|v| v.as_i64()),
            });
        }
    }
    if let Some(doc) = msg.get("document") {
        if let Some(file_id) = doc.get("file_id").and_then(|v| v.as_str()) {
            attachments.push(Attachment {
                id: Some(file_id.to_string()),
                url: format!("telegram://file/{}", file_id),
                mime_type: doc
                    .get("mime_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                filename: doc
                    .get("file_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                size: doc.get("file_size").and_then(|v| v.as_i64()),
            });
        }
    }

    Some(InboundMessage {
        inbound_id: msg_id.clone(),
        channel: "telegram".to_string(),
        account_id: None,
        peer_id: chat_id,
        peer_kind: peer_kind.to_string(),
        thread_id,
        message_id: Some(msg_id),
        sender_name: msg
            .get("from")
            .and_then(|v| v.get("username"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        text,
        attachments,
        timestamp: msg
            .get("date")
            .and_then(|v| v.as_i64())
            .map(|v| v.to_string()),
    })
}

pub async fn send_telegram_message(
    client: &Client,
    token: &str,
    chat_id: &str,
    text: Option<&str>,
    reply_to: Option<&str>,
    attachments: &[Attachment],
) -> Result<String> {
    if let Some(body) = text {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        let mut payload = serde_json::json!({
            "chat_id": chat_id,
            "text": body,
        });
        if let Some(reply) = reply_to {
            if let Ok(mid) = reply.parse::<i64>() {
                payload["reply_to_message_id"] = serde_json::Value::Number(mid.into());
            }
        }
        let resp = client.post(&url).json(&payload).send().await?;
        let value: Value = resp.json().await?;
        if value.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(anyhow::anyhow!("telegram send failed: {}", value));
        }
    }

    for attachment in attachments {
        if attachment.url.starts_with("telegram://file/") {
            continue;
        }
        let bytes = client.get(&attachment.url).send().await?.bytes().await?;
        let filename = attachment
            .filename
            .clone()
            .unwrap_or_else(|| "file".to_string());
        let url = format!("https://api.telegram.org/bot{}/sendDocument", token);
        let mut form = reqwest::multipart::Form::new()
            .text("chat_id", chat_id.to_string())
            .part(
                "document",
                reqwest::multipart::Part::bytes(bytes.to_vec()).file_name(filename),
            );
        if let Some(reply) = reply_to {
            form = form.text("reply_to_message_id", reply.to_string());
        }
        let resp = client.post(&url).multipart(form).send().await?;
        let value: Value = resp.json().await?;
        if value.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            return Err(anyhow::anyhow!("telegram document failed: {}", value));
        }
    }
    Ok("ok".to_string())
}

pub async fn resolve_telegram_file_url(
    client: &Client,
    token: &str,
    file_id: &str,
) -> Result<Option<String>> {
    let url = format!("https://api.telegram.org/bot{}/getFile", token);
    let resp = client
        .post(&url)
        .json(&serde_json::json!({"file_id": file_id}))
        .send()
        .await?;
    let value: Value = resp.json().await?;
    if value.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        return Ok(None);
    }
    let file_path = value
        .get("result")
        .and_then(|v| v.get("file_path"))
        .and_then(|v| v.as_str());
    Ok(file_path.map(|p| format!("https://api.telegram.org/file/bot{}/{}", token, p)))
}
