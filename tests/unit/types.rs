use agent_ping::types::{Attachment, InboundMessage, OutboundMessage, RouteInfo};
use serde_json;

#[test]
fn test_attachment_serde() {
    let att = Attachment {
        id: Some("123".to_string()),
        url: "https://example.com/file.pdf".to_string(),
        mime_type: Some("application/pdf".to_string()),
        filename: Some("document.pdf".to_string()),
        size: Some(1024),
    };

    let json = serde_json::to_string(&att).unwrap();
    let parsed: Attachment = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, att.id);
    assert_eq!(parsed.url, att.url);
}

#[test]
fn test_attachment_optional_fields() {
    let att = Attachment {
        id: None,
        url: "https://example.com/file.pdf".to_string(),
        mime_type: None,
        filename: None,
        size: None,
    };

    let json = serde_json::to_string(&att).unwrap();
    let parsed: Attachment = serde_json::from_str(&json).unwrap();
    assert!(parsed.id.is_none());
}

#[test]
fn test_inbound_message_serde() {
    let msg = InboundMessage {
        inbound_id: "inb_123".to_string(),
        channel: "slack".to_string(),
        account_id: Some("acc_456".to_string()),
        peer_id: "U789".to_string(),
        peer_kind: "dm".to_string(),
        thread_id: Some("thread_abc".to_string()),
        message_id: Some("msg_def".to_string()),
        sender_name: Some("John Doe".to_string()),
        text: Some("Hello!".to_string()),
        attachments: vec![],
        timestamp: Some("2024-01-01T00:00:00Z".to_string()),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: InboundMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.inbound_id, msg.inbound_id);
    assert_eq!(parsed.channel, msg.channel);
}

#[test]
fn test_inbound_message_minimal() {
    let msg = InboundMessage {
        inbound_id: "inb_minimal".to_string(),
        channel: "whatsapp".to_string(),
        account_id: None,
        peer_id: "+1234567890".to_string(),
        peer_kind: "dm".to_string(),
        thread_id: None,
        message_id: None,
        sender_name: None,
        text: None,
        attachments: vec![],
        timestamp: None,
    };

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: InboundMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.inbound_id, "inb_minimal");
    assert!(parsed.text.is_none());
}

#[test]
fn test_outbound_message_serde() {
    let msg = OutboundMessage {
        session_key: "agent:main:default".to_string(),
        text: Some("Hello from agent!".to_string()),
        attachments: vec![],
        channel: Some("slack".to_string()),
        account_id: Some("acc_123".to_string()),
        peer_id: Some("U456".to_string()),
        reply_to: Some("msg_789".to_string()),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: OutboundMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.session_key, msg.session_key);
    assert_eq!(parsed.text, msg.text);
}

#[test]
fn test_route_info_serde() {
    let route = RouteInfo {
        channel: "slack".to_string(),
        account_id: Some("acc_123".to_string()),
        peer_id: Some("U456".to_string()),
        thread_id: Some("thread_789".to_string()),
    };

    let json = serde_json::to_string(&route).unwrap();
    let parsed: RouteInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.channel, "slack");
}

#[test]
fn test_inbound_message_with_attachments() {
    let att = Attachment {
        id: Some("f1".to_string()),
        url: "https://example.com/image.jpg".to_string(),
        mime_type: Some("image/jpeg".to_string()),
        filename: Some("photo.jpg".to_string()),
        size: Some(2048),
    };

    let msg = InboundMessage {
        inbound_id: "inb_123".to_string(),
        channel: "telegram".to_string(),
        account_id: None,
        peer_id: "123456789".to_string(),
        peer_kind: "dm".to_string(),
        thread_id: None,
        message_id: Some("msg_123".to_string()),
        sender_name: None,
        text: Some("Check this out!".to_string()),
        attachments: vec![att.clone()],
        timestamp: None,
    };

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: InboundMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.attachments.len(), 1);
    assert_eq!(parsed.attachments[0].url, att.url);
}
