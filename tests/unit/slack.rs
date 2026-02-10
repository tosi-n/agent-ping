use agent_ping::channels::slack::parse_slack_event;
use serde_json::json;

#[test]
fn test_parse_dm_event() {
    let payload = json!({
        "type": "event_callback",
        "event": {
            "type": "message",
            "channel": "D1234",
            "user": "U12345",
            "text": "Hello world",
            "ts": "1234567890.123456"
        }
    });
    let event = parse_slack_event(&payload);
    assert!(event.is_some());
    let inbound = event.unwrap();
    assert_eq!(inbound.channel, "slack");
    assert_eq!(inbound.peer_id, "D1234");
    assert_eq!(inbound.peer_kind, "dm");
    assert_eq!(inbound.text, Some("Hello world".to_string()));
}

#[test]
fn test_parse_channel_message() {
    let payload = json!({
        "type": "event_callback",
        "event": {
            "type": "message",
            "channel": "C1234",
            "user": "U12345",
            "text": "Hello channel",
            "ts": "1234567890.123456"
        }
    });
    let event = parse_slack_event(&payload);
    assert!(event.is_some());
    let inbound = event.unwrap();
    assert_eq!(inbound.channel, "slack");
    assert_eq!(inbound.peer_id, "C1234");
    assert_eq!(inbound.peer_kind, "channel");
}

#[test]
fn test_ignore_bot_messages() {
    let payload = json!({
        "type": "event_callback",
        "event": {
            "type": "message",
            "channel": "C1234",
            "user": "U12345",
            "subtype": "bot_message",
            "text": "Bot message",
            "ts": "1234567890.123456"
        }
    });
    let event = parse_slack_event(&payload);
    assert!(event.is_none());
}

#[test]
fn test_parse_message_with_files() {
    let payload = json!({
        "type": "event_callback",
        "event": {
            "type": "message",
            "channel": "D1234",
            "user": "U12345",
            "text": "Message with file",
            "ts": "1234567890.123456",
            "files": [
                {
                    "id": "F1234",
                    "name": "test.txt",
                    "mimetype": "text/plain",
                    "size": 1024,
                    "url_private_download": "https://files.slack.com/files/test.txt"
                }
            ]
        }
    });
    let event = parse_slack_event(&payload);
    assert!(event.is_some());
    let inbound = event.unwrap();
    assert_eq!(inbound.attachments.len(), 1);
}

#[test]
fn test_parse_url_verification() {
    let payload = json!({
        "type": "url_verification",
        "challenge": "test_challenge_token"
    });
    let event = parse_slack_event(&payload);
    assert!(event.is_none());
}

#[test]
fn test_parse_message_with_thread() {
    let payload = json!({
        "type": "event_callback",
        "event": {
            "type": "message",
            "channel": "C1234",
            "user": "U12345",
            "text": "Thread reply",
            "thread_ts": "1234567890.123456",
            "ts": "1234567890.999999"
        }
    });
    let event = parse_slack_event(&payload);
    assert!(event.is_some());
    let inbound = event.unwrap();
    assert_eq!(inbound.thread_id, Some("1234567890.123456".to_string()));
}

#[test]
fn test_parse_message_blocks() {
    let payload = json!({
        "type": "event_callback",
        "event": {
            "type": "message",
            "channel": "D1234",
            "user": "U12345",
            "blocks": [
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": "Block message"
                    }
                }
            ],
            "ts": "1234567890.123456"
        }
    });
    let event = parse_slack_event(&payload);
    assert!(event.is_some());
    let inbound = event.unwrap();
    assert!(inbound.text.is_none() || inbound.text == Some("".to_string()));
}

#[test]
fn test_channel_id_case_preserved() {
    let payload = json!({
        "type": "event_callback",
        "event": {
            "type": "message",
            "channel": "C_UPPERCASE",
            "user": "U12345",
            "text": "Test",
            "ts": "1234567890.123456"
        }
    });
    let event = parse_slack_event(&payload);
    assert!(event.is_some());
    let inbound = event.unwrap();
    assert_eq!(inbound.peer_id, "C_UPPERCASE");
}

#[test]
fn test_user_id_case_preserved() {
    let payload = json!({
        "type": "event_callback",
        "event": {
            "type": "message",
            "channel": "D1234",
            "user": "U_UPPERCASE",
            "text": "Test",
            "ts": "1234567890.123456"
        }
    });
    let event = parse_slack_event(&payload);
    assert!(event.is_some());
    let inbound = event.unwrap();
    assert_eq!(inbound.sender_name, Some("U_UPPERCASE".to_string()));
}

#[test]
fn test_empty_text_handling() {
    let payload = json!({
        "type": "event_callback",
        "event": {
            "type": "message",
            "channel": "D1234",
            "user": "U12345",
            "text": "",
            "ts": "1234567890.123456"
        }
    });
    let event = parse_slack_event(&payload);
    assert!(event.is_some());
    let inbound = event.unwrap();
    assert_eq!(inbound.text, Some("".to_string()));
}

#[test]
fn test_parse_non_message_event() {
    let payload = json!({
        "type": "event_callback",
        "event": {
            "type": "reaction_added",
            "channel": "C1234",
            "user": "U12345",
            "ts": "1234567890.123456"
        }
    });
    let event = parse_slack_event(&payload);
    assert!(event.is_none());
}
