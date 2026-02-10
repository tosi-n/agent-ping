use agent_ping::ws::{WsCommand, WsEvent};
use serde_json::json;

#[test]
fn test_ws_event_serde() {
    let event = WsEvent {
        event: "test".to_string(),
        payload: json!({"key": "value"}),
    };
    let json = serde_json::to_string(&event).unwrap();
    let parsed: WsEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.event, "test");
    assert_eq!(parsed.payload["key"], "value");
}

#[test]
fn test_ws_command_connect() {
    let cmd_json = json!({
        "type": "connect",
        "token": "test-token"
    });
    let cmd: WsCommand = serde_json::from_value(cmd_json).unwrap();
    match cmd {
        WsCommand::Connect { token } => {
            assert_eq!(token, Some("test-token".to_string()));
        }
        _ => panic!("Expected Connect command"),
    }
}

#[test]
fn test_ws_command_connect_no_token() {
    let cmd_json = json!({
        "type": "connect"
    });
    let cmd: WsCommand = serde_json::from_value(cmd_json).unwrap();
    match cmd {
        WsCommand::Connect { token } => {
            assert!(token.is_none());
        }
        _ => panic!("Expected Connect command"),
    }
}

#[test]
fn test_ws_command_subscribe() {
    let cmd_json = json!({
        "type": "subscribe",
        "events": ["chat", "delivery"]
    });
    let cmd: WsCommand = serde_json::from_value(cmd_json).unwrap();
    match cmd {
        WsCommand::Subscribe { events } => {
            assert_eq!(
                events,
                Some(vec!["chat".to_string(), "delivery".to_string()])
            );
        }
        _ => panic!("Expected Subscribe command"),
    }
}

#[test]
fn test_ws_command_subscribe_empty() {
    let cmd_json = json!({
        "type": "subscribe",
        "events": []
    });
    let cmd: WsCommand = serde_json::from_value(cmd_json).unwrap();
    match cmd {
        WsCommand::Subscribe { events } => {
            assert_eq!(events, Some(vec![]));
        }
        _ => panic!("Expected Subscribe command"),
    }
}

#[test]
fn test_ws_command_ping() {
    let cmd_json = json!({
        "type": "ping"
    });
    let cmd: WsCommand = serde_json::from_value(cmd_json).unwrap();
    match cmd {
        WsCommand::Ping => {}
        _ => panic!("Expected Ping command"),
    }
}

#[test]
fn test_ws_command_unknown_type() {
    let cmd_json = json!({
        "type": "unknown"
    });
    let result = serde_json::from_value::<WsCommand>(cmd_json);
    assert!(result.is_err());
}

#[test]
fn test_ws_event_variants() {
    let chat_event = WsEvent {
        event: "chat".to_string(),
        payload: json!({"direction": "inbound"}),
    };
    assert_eq!(chat_event.event, "chat");

    let delivery_event = WsEvent {
        event: "delivery".to_string(),
        payload: json!({"status": "sent"}),
    };
    assert_eq!(delivery_event.event, "delivery");
}
