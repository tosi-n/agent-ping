use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEvent {
    pub event: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsCommand {
    #[serde(rename = "connect")]
    Connect { token: Option<String> },
    #[serde(rename = "subscribe")]
    Subscribe { events: Option<Vec<String>> },
    #[serde(rename = "ping")]
    Ping,
}

pub async fn handle_ws(
    mut socket: WebSocket,
    mut rx: broadcast::Receiver<WsEvent>,
    auth_token: Option<String>,
) {
    let mut authorized = auth_token.is_none();
    let mut subscriptions: Option<HashSet<String>> = None;

    loop {
        tokio::select! {
            msg = socket.recv() => {
                if msg.is_none() {
                    break;
                }
                if let Some(Ok(Message::Close(_))) = msg {
                    break;
                }
                if let Some(Ok(Message::Text(text))) = msg {
                    if let Ok(cmd) = serde_json::from_str::<WsCommand>(&text) {
                        match cmd {
                            WsCommand::Connect { token } => {
                                if let Some(expected) = auth_token.as_ref() {
                                    if token.as_deref() != Some(expected.as_str()) {
                                        let _ = socket.send(Message::Close(None)).await;
                                        break;
                                    }
                                }
                                authorized = true;
                                let ack = WsEvent {
                                    event: "presence".to_string(),
                                    payload: serde_json::json!({"status": "connected"}),
                                };
                                let _ = socket.send(Message::Text(serde_json::to_string(&ack).unwrap_or_default())).await;
                            }
                            WsCommand::Subscribe { events } => {
                                subscriptions = events.map(|items| items.into_iter().collect());
                            }
                            WsCommand::Ping => {
                                let health = WsEvent {
                                    event: "health".to_string(),
                                    payload: serde_json::json!({"status": "ok"}),
                                };
                                let _ = socket.send(Message::Text(serde_json::to_string(&health).unwrap_or_default())).await;
                            }
                        }
                    }
                }
            }
            evt = rx.recv() => {
                if let Ok(evt) = evt {
                    if !authorized {
                        continue;
                    }
                    if let Some(subs) = subscriptions.as_ref() {
                        if !subs.contains(&evt.event) {
                            continue;
                        }
                    }
                    let text = serde_json::to_string(&evt).unwrap_or_default();
                    if socket.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ws_event_serialize() {
        let event = WsEvent {
            event: "test".to_string(),
            payload: json!({"key": "value"}),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\":\"test\""));
        assert!(json.contains("\"key\":\"value\""));
    }

    #[test]
    fn test_ws_command_connect_serialize() {
        let cmd = WsCommand::Connect {
            token: Some("token123".to_string()),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"connect\""));
        assert!(json.contains("\"token\":\"token123\""));
    }

    #[test]
    fn test_ws_command_ping_serialize() {
        let cmd = WsCommand::Ping;
        let json = serde_json::to_string(&cmd).unwrap();
        assert_eq!(json, r#"{"type":"ping"}"#);
    }

    #[test]
    fn test_ws_command_subscribe_serialize() {
        let cmd = WsCommand::Subscribe {
            events: Some(vec!["a".to_string(), "b".to_string()]),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"subscribe\""));
        assert!(json.contains("\"events\""));
    }

    #[test]
    fn test_ws_command_deserialize_connect() {
        let json = r#"{"type":"connect","token":"my_token"}"#;
        let cmd: WsCommand = serde_json::from_str(json).unwrap();
        match cmd {
            WsCommand::Connect { token } => assert_eq!(token, Some("my_token".to_string())),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_ws_command_deserialize_subscribe() {
        let json = r#"{"type":"subscribe","events":["chat"]}"#;
        let cmd: WsCommand = serde_json::from_str(json).unwrap();
        match cmd {
            WsCommand::Subscribe { events } => {
                assert!(events.is_some());
                assert_eq!(events.unwrap().len(), 1);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_ws_command_deserialize_ping() {
        let json = r#"{"type":"ping"}"#;
        let cmd: WsCommand = serde_json::from_str(json).unwrap();
        match cmd {
            WsCommand::Ping => {}
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_ws_event_with_null_payload() {
        let event = WsEvent {
            event: "test".to_string(),
            payload: serde_json::Value::Null,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\":\"test\""));
        assert!(json.contains("null"));
    }

    #[test]
    fn test_ws_command_connect_no_token() {
        let cmd = WsCommand::Connect { token: None };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"connect\""));
    }

    #[test]
    fn test_ws_command_subscribe_empty_events() {
        let cmd = WsCommand::Subscribe { events: Some(vec![]) };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"subscribe\""));
        assert!(json.contains("\"events\":[]"));
    }

    #[test]
    fn test_ws_command_subscribe_no_events() {
        let cmd = WsCommand::Subscribe { events: None };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"subscribe\""));
    }

    #[test]
    fn test_ws_event_array_payload() {
        let event = WsEvent {
            event: "list".to_string(),
            payload: json!([1, 2, 3]),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\":\"list\""));
    }

    #[test]
    fn test_ws_event_nested_payload() {
        let event = WsEvent {
            event: "nested".to_string(),
            payload: json!({"outer": {"inner": "value"}}),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\":\"nested\""));
    }

    #[test]
    fn test_ws_command_deserialize_connect_no_token() {
        let json = r#"{"type":"connect"}"#;
        let cmd: WsCommand = serde_json::from_str(json).unwrap();
        match cmd {
            WsCommand::Connect { token } => assert!(token.is_none()),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_ws_command_deserialize_subscribe_empty() {
        let json = r#"{"type":"subscribe","events":[]}"#;
        let cmd: WsCommand = serde_json::from_str(json).unwrap();
        match cmd {
            WsCommand::Subscribe { events } => assert!(events.unwrap().is_empty()),
            _ => panic!("wrong variant"),
        }
    }
}
