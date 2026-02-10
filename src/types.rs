use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Option<String>,
    pub url: String,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    pub size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub inbound_id: String,
    pub channel: String,
    pub account_id: Option<String>,
    pub peer_id: String,
    pub peer_kind: String,
    pub thread_id: Option<String>,
    pub message_id: Option<String>,
    pub sender_name: Option<String>,
    pub text: Option<String>,
    pub attachments: Vec<Attachment>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub session_key: String,
    pub text: Option<String>,
    pub attachments: Vec<Attachment>,
    pub channel: Option<String>,
    pub account_id: Option<String>,
    pub peer_id: Option<String>,
    pub reply_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub channel: String,
    pub account_id: Option<String>,
    pub peer_id: Option<String>,
    pub thread_id: Option<String>,
}
