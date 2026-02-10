use crate::types::{Attachment, InboundMessage};
use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use serde_json::Value;

pub async fn send_slack_message(
    client: &Client,
    token: &str,
    channel: &str,
    text: Option<&str>,
    thread_ts: Option<&str>,
    attachments: &[Attachment],
) -> Result<String> {
    if let Some(body) = text {
        let mut payload = serde_json::json!({
            "channel": channel,
            "text": body,
        });
        if let Some(ts) = thread_ts {
            payload["thread_ts"] = serde_json::Value::String(ts.to_string());
        }

        let resp = client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        let value: Value = resp.json().await?;
        if !value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            return Err(anyhow::anyhow!("slack send failed: {}", value));
        }
    }

    for attachment in attachments {
        let filename = attachment
            .filename
            .clone()
            .unwrap_or_else(|| "file".to_string());
        let bytes = client.get(&attachment.url).send().await?.bytes().await?;
        let mut form = reqwest::multipart::Form::new()
            .part(
                "file",
                reqwest::multipart::Part::bytes(bytes.to_vec()).file_name(filename),
            )
            .text("channels", channel.to_string());

        if let Some(ts) = thread_ts {
            form = form.text("thread_ts", ts.to_string());
        }

        let resp = client
            .post("https://slack.com/api/files.upload")
            .bearer_auth(token)
            .multipart(form)
            .send()
            .await?;
        let value: Value = resp.json().await?;
        if !value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            return Err(anyhow::anyhow!("slack upload failed: {}", value));
        }
    }

    Ok("ok".to_string())
}

pub fn parse_slack_event(payload: &Value) -> Option<InboundMessage> {
    let event_type = payload.get("type")?.as_str()?;
    if event_type == "url_verification" || event_type != "event_callback" {
        return None;
    }

    let event = payload.get("event")?;
    if event.get("type")?.as_str()? != "message" {
        return None;
    }
    if event.get("subtype").is_some() {
        return None;
    }

    let channel = event.get("channel")?.as_str()?.to_string();
    let text = event
        .get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let ts = event
        .get("ts")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let thread_ts = event
        .get("thread_ts")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let peer_kind = if channel.starts_with('D') { "dm" } else { "channel" };

    let mut attachments = Vec::new();
    if let Some(files) = event.get("files").and_then(|v| v.as_array()) {
        for file in files {
            if let Some(url) = file.get("url_private_download").and_then(|v| v.as_str()) {
                attachments.push(Attachment {
                    id: file
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    url: url.to_string(),
                    mime_type: file
                        .get("mimetype")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    filename: file
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    size: file.get("size").and_then(|v| v.as_i64()),
                });
            }
        }
    }

    Some(InboundMessage {
        inbound_id: ts
            .clone()
            .unwrap_or_else(|| Utc::now().timestamp_millis().to_string()),
        channel: "slack".to_string(),
        account_id: None,
        peer_id: channel,
        peer_kind: peer_kind.to_string(),
        thread_id: thread_ts,
        message_id: ts,
        sender_name: event
            .get("user")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        text,
        attachments,
        timestamp: event
            .get("event_ts")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}
