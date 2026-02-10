use agent_ping::channels::telegram::parse_telegram_update;
use serde_json::json;

#[test]
fn test_parse_telegram_private_message() {
    let payload = json!({
        "update_id": 123456789,
        "message": {
            "message_id": 1,
            "from": {
                "id": 123456789_i64,
                "is_bot": false,
                "first_name": "Test",
                "username": "testuser"
            },
            "chat": {
                "id": 123456789_i64,
                "type": "private",
                "first_name": "Test",
                "username": "testuser"
            },
            "date": 1609459200,
            "text": "Hello from Telegram"
        }
    });
    let update = parse_telegram_update(&payload);
    assert!(update.is_some());
    let inbound = update.unwrap();
    assert_eq!(inbound.channel, "telegram");
    assert_eq!(inbound.peer_id, "123456789");
    assert_eq!(inbound.peer_kind, "dm");
    assert_eq!(inbound.text, Some("Hello from Telegram".to_string()));
}

#[test]
fn test_parse_telegram_group_message() {
    let payload = json!({
        "update_id": 123456789,
        "message": {
            "message_id": 1,
            "from": {
                "id": 123456789_i64,
                "is_bot": false,
                "first_name": "Test",
                "username": "testuser"
            },
            "chat": {
                "id": -1001234567890_i64,
                "type": "supergroup",
                "title": "Test Group"
            },
            "date": 1609459200,
            "text": "Hello from group"
        }
    });
    let update = parse_telegram_update(&payload);
    assert!(update.is_some());
    let inbound = update.unwrap();
    assert_eq!(inbound.channel, "telegram");
    assert_eq!(inbound.peer_id, "-1001234567890");
    assert_eq!(inbound.peer_kind, "group");
}

#[test]
fn test_ignore_telegram_channel_posts() {
    let payload = json!({
        "update_id": 123456789,
        "channel_post": {
            "message_id": 1,
            "chat": {
                "id": -1001234567890_i64,
                "type": "channel"
            },
            "date": 1609459200,
            "text": "Channel post"
        }
    });
    let update = parse_telegram_update(&payload);
    assert!(update.is_some());
}

#[test]
fn test_parse_telegram_photo() {
    let payload = json!({
        "update_id": 123456789,
        "message": {
            "message_id": 1,
            "from": {
                "id": 123456789_i64,
                "is_bot": false,
                "first_name": "Test"
            },
            "chat": {
                "id": 123456789_i64,
                "type": "private"
            },
            "date": 1609459200,
            "photo": [
                {"file_id": "photo_small", "width": 160, "height": 160},
                {"file_id": "photo_medium", "width": 320, "height": 320},
                {"file_id": "photo_large", "width": 640, "height": 640}
            ]
        }
    });
    let update = parse_telegram_update(&payload);
    assert!(update.is_some());
    let inbound = update.unwrap();
    assert_eq!(inbound.attachments.len(), 1);
    assert_eq!(inbound.attachments[0].id, Some("photo_large".to_string()));
}

#[test]
fn test_parse_telegram_document() {
    let payload = json!({
        "update_id": 123456789,
        "message": {
            "message_id": 1,
            "from": {
                "id": 123456789_i64,
                "is_bot": false,
                "first_name": "Test"
            },
            "chat": {
                "id": 123456789_i64,
                "type": "private"
            },
            "date": 1609459200,
            "document": {
                "file_id": "doc123",
                "file_name": "test.pdf",
                "mime_type": "application/pdf"
            }
        }
    });
    let update = parse_telegram_update(&payload);
    assert!(update.is_some());
    let inbound = update.unwrap();
    assert_eq!(inbound.attachments.len(), 1);
    assert_eq!(inbound.attachments[0].id, Some("doc123".to_string()));
    assert_eq!(
        inbound.attachments[0].filename,
        Some("test.pdf".to_string())
    );
}

#[test]
fn test_ignore_telegram_callback_query() {
    let payload = json!({
        "update_id": 123456789,
        "callback_query": {
            "id": "callback123",
            "from": {
                "id": 123456789_i64,
                "is_bot": false,
                "first_name": "Test"
            },
            "message": {
                "message_id": 1,
                "chat": {
                    "id": 123456789_i64,
                    "type": "private"
                },
                "date": 1609459200
            },
            "data": "button_clicked"
        }
    });
    let update = parse_telegram_update(&payload);
    assert!(update.is_none());
}

#[test]
fn test_parse_telegram_empty_message() {
    let payload = json!({
        "update_id": 123456789,
        "message": {
            "message_id": 1,
            "from": {
                "id": 123456789_i64,
                "is_bot": false,
                "first_name": "Test"
            },
            "chat": {
                "id": 123456789_i64,
                "type": "private"
            },
            "date": 1609459200
        }
    });
    let update = parse_telegram_update(&payload);
    assert!(update.is_some());
    let inbound = update.unwrap();
    assert!(inbound.text.is_none());
}

#[test]
fn test_parse_telegram_with_caption() {
    let payload = json!({
        "update_id": 123456789,
        "message": {
            "message_id": 1,
            "from": {
                "id": 123456789_i64,
                "is_bot": false,
                "first_name": "Test"
            },
            "chat": {
                "id": 123456789_i64,
                "type": "private"
            },
            "date": 1609459200,
            "photo": [
                {"file_id": "photo123", "width": 320, "height": 320}
            ],
            "caption": "Photo caption"
        }
    });
    let update = parse_telegram_update(&payload);
    assert!(update.is_some());
    let inbound = update.unwrap();
    assert_eq!(inbound.attachments.len(), 1);
    assert!(inbound.text.is_none());
}

#[test]
fn test_ignore_telegram_service_messages() {
    let payload = json!({
        "update_id": 123456789,
        "message": {
            "message_id": 1,
            "chat": {
                "id": 123456789_i64,
                "type": "private"
            },
            "date": 1609459200,
            "new_chat_participant": {
                "id": 987654321_i64,
                "is_bot": true,
                "first_name": "Bot"
            }
        }
    });
    let update = parse_telegram_update(&payload);
    assert!(update.is_some());
}

#[test]
fn test_parse_telegram_location() {
    let payload = json!({
        "update_id": 123456789,
        "message": {
            "message_id": 1,
            "from": {
                "id": 123456789_i64,
                "is_bot": false,
                "first_name": "Test"
            },
            "chat": {
                "id": 123456789_i64,
                "type": "private"
            },
            "date": 1609459200,
            "location": {
                "latitude": 40.7128_f64,
                "longitude": -74.0060_f64
            }
        }
    });
    let update = parse_telegram_update(&payload);
    assert!(update.is_some());
    let inbound = update.unwrap();
    assert!(inbound.text.is_none());
}

#[test]
fn test_parse_telegram_voice() {
    let payload = json!({
        "update_id": 123456789,
        "message": {
            "message_id": 1,
            "from": {
                "id": 123456789_i64,
                "is_bot": false,
                "first_name": "Test"
            },
            "chat": {
                "id": 123456789_i64,
                "type": "private"
            },
            "date": 1609459200,
            "voice": {
                "file_id": "voice123",
                "duration": 30,
                "mime_type": "audio/ogg"
            }
        }
    });
    let update = parse_telegram_update(&payload);
    assert!(update.is_some());
    let inbound = update.unwrap();
    assert!(inbound.attachments.is_empty());
}
