use agent_ping::ws::{WsCommand, WsEvent};
use serde_json::json;

#[test]
fn test_ws_command_connect() {
    let cmd = WsCommand::Connect {
        token: Some("test_token_123".to_string()),
    };
    let json = serde_json::to_string(&cmd).unwrap();
    assert!(json.contains("\"type\":\"connect\""));
    assert!(json.contains("\"token\":\"test_token_123\""));
}

#[test]
fn test_ws_command_connect_without_token() {
    let cmd = WsCommand::Connect { token: None };
    let json = serde_json::to_string(&cmd).unwrap();
    assert!(json.contains("\"type\":\"connect\""));
}

#[test]
fn test_ws_command_ping() {
    let cmd = WsCommand::Ping;
    let json = serde_json::to_string(&cmd).unwrap();
    assert!(json.contains("\"type\":\"ping\""));
}

#[test]
fn test_ws_command_subscribe_multiple() {
    let cmd = WsCommand::Subscribe {
        events: Some(vec!["chat".to_string(), "status".to_string()]),
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["type"], "subscribe");
    assert!(parsed["events"].is_array());
    assert_eq!(parsed["events"].as_array().unwrap().len(), 2);
}

#[test]
fn test_ws_command_subscribe_empty() {
    let cmd = WsCommand::Subscribe {
        events: Some(vec![]),
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["events"].as_array().unwrap().len(), 0);
}

#[test]
fn test_ws_event_chat() {
    let event = WsEvent {
        event: "chat".to_string(),
        payload: json!({
            "direction": "inbound",
            "message": {"id": "msg_123", "content": "Hello"}
        }),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"event\":\"chat\""));
    assert!(json.contains("\"direction\":\"inbound\""));
}

#[test]
fn test_ws_event_status() {
    let event = WsEvent {
        event: "status".to_string(),
        payload: json!({"connected": true, "sessions": 10}),
    };
    let json = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["event"], "status");
    assert_eq!(parsed["payload"]["connected"], true);
}

#[test]
fn test_ws_event_variants() {
    let chat = WsEvent {
        event: "chat".to_string(),
        payload: json!({}),
    };
    let status = WsEvent {
        event: "status".to_string(),
        payload: json!({}),
    };
    let error = WsEvent {
        event: "error".to_string(),
        payload: json!({"message": "test error"}),
    };

    assert_eq!(chat.event, "chat");
    assert_eq!(status.event, "status");
    assert_eq!(error.event, "error");
}

#[test]
fn test_ws_command_deserialization_connect() {
    let json = r#"{"type":"connect","token":"abc123"}"#;
    let cmd: WsCommand = serde_json::from_str(json).unwrap();
    match cmd {
        WsCommand::Connect { token } => {
            assert_eq!(token, Some("abc123".to_string()));
        }
        _ => panic!("Expected Connect variant"),
    }
}

#[test]
fn test_ws_command_deserialization_ping() {
    let json = r#"{"type":"ping"}"#;
    let cmd: WsCommand = serde_json::from_str(json).unwrap();
    match cmd {
        WsCommand::Ping => {}
        _ => panic!("Expected Ping variant"),
    }
}

#[test]
fn test_ws_command_deserialization_subscribe() {
    let json = r#"{"type":"subscribe","events":["chat","status"]}"#;
    let cmd: WsCommand = serde_json::from_str(json).unwrap();
    match cmd {
        WsCommand::Subscribe { events } => {
            assert!(events.is_some());
            assert_eq!(events.unwrap().len(), 2);
        }
        _ => panic!("Expected Subscribe variant"),
    }
}

#[test]
fn test_ws_event_deserialization() {
    let json = r#"{"event":"chat","payload":{"msg":"test"}}"#;
    let event: WsEvent = serde_json::from_str(json).unwrap();
    assert_eq!(event.event, "chat");
    assert_eq!(event.payload["msg"], "test");
}

#[test]
fn test_ws_command_connect_null_token() {
    let json = r#"{"type":"connect","token":null}"#;
    let cmd: WsCommand = serde_json::from_str(json).unwrap();
    match cmd {
        WsCommand::Connect { token } => {
            assert!(token.is_none());
        }
        _ => panic!("Expected Connect variant"),
    }
}

#[test]
fn test_ws_event_with_complex_payload() {
    let event = WsEvent {
        event: "message".to_string(),
        payload: json!({
            "id": "msg_001",
            "direction": "outbound",
            "channel": "slack",
            "content": "Test message",
            "attachments": [],
            "metadata": {
                "source": "api",
                "version": "1.0"
            }
        }),
    };
    let json = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["event"], "message");
    assert_eq!(parsed["payload"]["direction"], "outbound");
    assert_eq!(parsed["payload"]["channel"], "slack");
    assert_eq!(parsed["payload"]["metadata"]["version"], "1.0");
}

#[test]
fn test_ws_event_empty_payload() {
    let event = WsEvent {
        event: "test".to_string(),
        payload: json!(null),
    };
    let json = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["event"], "test");
    assert!(parsed["payload"].is_null());
}

#[test]
fn test_ws_command_subscribe_none_events() {
    let json = r#"{"type":"subscribe"}"#;
    let cmd: WsCommand = serde_json::from_str(json).unwrap();
    match cmd {
        WsCommand::Subscribe { events } => {
            assert!(events.is_none());
        }
        _ => panic!("Expected Subscribe variant"),
    }
}
