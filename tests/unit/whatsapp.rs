use agent_ping::channels::whatsapp::{normalize_whatsapp_inbound, WhatsAppInboundPayload};
use agent_ping::types::Attachment;

#[test]
fn test_normalize_whatsapp_inbound_basic() {
    let payload = WhatsAppInboundPayload {
        peer_id: "1234567890".to_string(),
        text: Some("Hello WhatsApp".to_string()),
        message_id: Some("msg123".to_string()),
        thread_id: None,
        attachments: None,
        sender_name: Some("Test User".to_string()),
    };
    let inbound = normalize_whatsapp_inbound(payload);
    assert_eq!(inbound.channel, "whatsapp");
    assert_eq!(inbound.peer_id, "1234567890");
    assert_eq!(inbound.peer_kind, "dm");
    assert_eq!(inbound.text, Some("Hello WhatsApp".to_string()));
}

#[test]
fn test_normalize_whatsapp_with_attachments() {
    let payload = WhatsAppInboundPayload {
        peer_id: "1234567890".to_string(),
        text: Some("Check this out".to_string()),
        message_id: Some("msg123".to_string()),
        thread_id: None,
        attachments: Some(vec![Attachment {
            id: Some("image123".to_string()),
            url: "https://example.com/image.jpg".to_string(),
            mime_type: Some("image/jpeg".to_string()),
            filename: None,
            size: None,
        }]),
        sender_name: Some("Test User".to_string()),
    };
    let inbound = normalize_whatsapp_inbound(payload);
    assert_eq!(inbound.channel, "whatsapp");
    assert_eq!(inbound.peer_id, "1234567890");
    assert_eq!(inbound.attachments.len(), 1);
    assert_eq!(inbound.attachments[0].id, Some("image123".to_string()));
}

#[test]
fn test_normalize_whatsapp_empty_text() {
    let payload = WhatsAppInboundPayload {
        peer_id: "1234567890".to_string(),
        text: None,
        message_id: Some("msg123".to_string()),
        thread_id: None,
        attachments: None,
        sender_name: None,
    };
    let inbound = normalize_whatsapp_inbound(payload);
    assert_eq!(inbound.channel, "whatsapp");
    assert!(inbound.text.is_none());
}

#[test]
fn test_normalize_whatsapp_with_thread() {
    let payload = WhatsAppInboundPayload {
        peer_id: "1234567890".to_string(),
        text: Some("Thread reply".to_string()),
        message_id: Some("msg456".to_string()),
        thread_id: Some("msg123".to_string()),
        attachments: None,
        sender_name: None,
    };
    let inbound = normalize_whatsapp_inbound(payload);
    assert_eq!(inbound.channel, "whatsapp");
    assert_eq!(inbound.thread_id, Some("msg123".to_string()));
}

#[test]
fn test_normalize_whatsapp_message_id_generation() {
    let payload = WhatsAppInboundPayload {
        peer_id: "1234567890".to_string(),
        text: Some("Test".to_string()),
        message_id: None,
        thread_id: None,
        attachments: None,
        sender_name: None,
    };
    let inbound = normalize_whatsapp_inbound(payload);
    assert!(!inbound.inbound_id.is_empty());
}

#[test]
fn test_normalize_whatsapp_no_attachments() {
    let payload = WhatsAppInboundPayload {
        peer_id: "1234567890".to_string(),
        text: Some("Text only".to_string()),
        message_id: Some("msg123".to_string()),
        thread_id: None,
        attachments: None,
        sender_name: None,
    };
    let inbound = normalize_whatsapp_inbound(payload);
    assert!(inbound.attachments.is_empty());
}
