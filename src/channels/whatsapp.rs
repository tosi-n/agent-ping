use crate::types::{Attachment, InboundMessage};
use anyhow::Result;
use reqwest::Client;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WhatsAppInboundPayload {
    pub peer_id: String,
    pub text: Option<String>,
    pub message_id: Option<String>,
    pub thread_id: Option<String>,
    pub attachments: Option<Vec<Attachment>>,
    pub sender_name: Option<String>,
}

pub async fn send_whatsapp_message(
    client: &Client,
    sidecar_url: &str,
    to: &str,
    text: Option<&str>,
    attachments: &[Attachment],
) -> Result<String> {
    let payload = serde_json::json!({
        "to": to,
        "text": text,
        "attachments": attachments,
    });
    let resp = client
        .post(format!("{}/send", sidecar_url))
        .json(&payload)
        .send()
        .await?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("whatsapp sidecar error: {}", body));
    }
    Ok("ok".to_string())
}

pub fn normalize_whatsapp_inbound(payload: WhatsAppInboundPayload) -> InboundMessage {
    InboundMessage {
        inbound_id: payload
            .message_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        channel: "whatsapp".to_string(),
        account_id: None,
        peer_id: payload.peer_id,
        peer_kind: "dm".to_string(),
        thread_id: payload.thread_id,
        message_id: payload.message_id,
        sender_name: payload.sender_name,
        text: payload.text,
        attachments: payload.attachments.unwrap_or_default(),
        timestamp: None,
    }
}
