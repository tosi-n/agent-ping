use agent_ping::config::{
    AuthConfig, BackendConfig, Binding, ChannelsConfig, Config, DatabaseConfig, QueueConfig,
    ServerConfig, SessionConfig, SlackConfig, TelegramConfig, WhatsAppConfig,
};
use agent_ping::db::{self, DbKind};
use agent_ping::types::InboundMessage;
use agent_ping::AppState;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use sqlx::AnyPool;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

fn create_test_config() -> Config {
    Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
        },
        auth: AuthConfig {
            token: Some("test_token_123".to_string()),
        },
        database: DatabaseConfig {
            url: None,
            sqlite_path: "~/.agent-ping/state.sqlite".to_string(),
        },
        backend: BackendConfig {
            webhook_url: None,
            media_upload_url: None,
            api_token: None,
        },
        session: SessionConfig {
            agent_id: "test_agent".to_string(),
            dm_scope: "per-peer".to_string(),
            main_key: "main".to_string(),
            identity_links: HashMap::new(),
        },
        queue: QueueConfig {
            mode: "collect".to_string(),
            debounce_ms: 1000,
            cap: 20,
            drop: "summarize".to_string(),
        },
        channels: ChannelsConfig {
            slack: SlackConfig {
                enabled: true,
                bot_token: Some("xoxb-test-token".to_string()),
                signing_secret: None,
                app_token: None,
                mode: "http".to_string(),
                webhook_path: "/v1/channels/slack/events".to_string(),
            },
            telegram: TelegramConfig {
                enabled: false,
                bot_token: None,
                poll_interval_seconds: 2,
            },
            whatsapp: WhatsAppConfig {
                enabled: false,
                sidecar_url: "http://127.0.0.1:4040".to_string(),
                inbound_path: "/v1/channels/whatsapp/inbound".to_string(),
            },
        },
        bindings: vec![
            Binding {
                channel: "slack".to_string(),
                account_id: Some("C123".to_string()),
                peer_id: None,
                business_profile_id: Some("bp_123".to_string()),
                user_id: None,
                agent_id: None,
            },
        ],
    }
}

async fn create_test_app_state() -> (AppState, TempDir) {
    sqlx::any::install_default_drivers();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite:///{}", db_path.to_string_lossy());

    let pool = AnyPool::connect(&db_url).await.unwrap();
    let kind = DbKind::Sqlite;
    db::init_db(&pool, kind).await.unwrap();

    let config = create_test_config();
    let (ws_tx, _) = broadcast::channel(100);

    let state = AppState {
        config: config.clone(),
        pool,
        http: reqwest::Client::new(),
        ws_tx,
        db_kind: kind,
    };

    (state, temp_dir)
}

fn create_app(state: &AppState) -> Router {
    use axum::middleware;

    let authed_routes = Router::new()
        .route("/v1/sessions", get(list_sessions))
        .route("/v1/sessions/:session_key", get(get_session))
        .route("/v1/sessions/:session_key/messages", get(list_messages))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let public_routes = Router::new()
        .route("/v1/health", get(health))
        .route("/v1/status", get(status_handler))
        .route(&state.config.channels.slack.webhook_path, post(slack_events))
        .route(&state.config.channels.whatsapp.inbound_path, post(whatsapp_inbound));

    Router::new()
        .merge(authed_routes)
        .merge(public_routes)
        .with_state(state.clone())
}

async fn health() -> impl axum::response::IntoResponse {
    Json(json!({"status": "ok"}))
}

async fn status_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    let sessions = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessions")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    let messages = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM messages")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    Json(json!({"sessions": sessions, "messages": messages}))
}

async fn list_sessions(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let limit = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(100);
    let offset = params.get("offset").and_then(|v| v.parse().ok()).unwrap_or(0);
    let sessions = db::list_sessions(&state.pool, state.db_kind, limit as i64, offset as i64)
        .await
        .unwrap_or_default();
    Json(sessions)
}

async fn get_session(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(session_key): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    let session = db::get_session(&state.pool, state.db_kind, &session_key)
        .await
        .unwrap_or(None);
    if let Some(session) = session {
        Json(session).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn list_messages(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(session_key): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let limit = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(200);
    let offset = params.get("offset").and_then(|v| v.parse().ok()).unwrap_or(0);
    let messages = db::list_messages(&state.pool, state.db_kind, &session_key, limit as i64, offset as i64)
        .await
        .unwrap_or_default();
    Json(messages)
}

async fn require_auth(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    if let Some(token) = state.config.auth.token.as_ref() {
        let header = headers
            .get("X-Agent-Ping-Token")
            .and_then(|v| v.to_str().ok());
        if header != Some(token.as_str()) {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }
    next.run(req).await
}

async fn slack_events(
    axum::extract::State(_state): axum::extract::State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    if payload.get("type").and_then(|v| v.as_str()) == Some("url_verification") {
        if let Some(challenge) = payload.get("challenge").and_then(|v| v.as_str()) {
            return Json(json!({"challenge": challenge}));
        }
    }
    Json(json!({"ok": true}))
}

async fn whatsapp_inbound(
    axum::extract::State(_state): axum::extract::State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    let _inbound = InboundMessage {
        inbound_id: format!("wa_{}", uuid::Uuid::new_v4()),
        channel: "whatsapp".to_string(),
        account_id: Some("business_123".to_string()),
        peer_id: "+1234567890".to_string(),
        peer_kind: "private".to_string(),
        thread_id: None,
        message_id: Some(format!("wa_msg_{}", uuid::Uuid::new_v4())),
        sender_name: Some("Test User".to_string()),
        text: payload.get("message").and_then(|v| v.as_str()).map(|s| s.to_string()),
        attachments: vec![],
        timestamp: None,
    };
    Json(json!({"status": "accepted"}))
}

#[tokio::test]
async fn test_health_endpoint() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_status_endpoint() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_status_returns_zero_for_empty_db() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["sessions"], 0);
    assert_eq!(value["messages"], 0);
}

#[tokio::test]
async fn test_slack_url_verification() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let payload = json!({
        "type": "url_verification",
        "challenge": "test_challenge_value"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/channels/slack/events")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["challenge"], "test_challenge_value");
}

#[tokio::test]
async fn test_slack_event_other() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let payload = json!({
        "type": "event_callback",
        "event": {"type": "message"}
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/channels/slack/events")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_sessions_empty() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/sessions")
                .header("X-Agent-Ping-Token", "test_token_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_sessions_with_pagination() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/sessions?limit=10&offset=5")
                .header("X-Agent-Ping-Token", "test_token_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_session_not_found() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/sessions/nonexistent")
                .header("X-Agent-Ping-Token", "test_token_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_messages_empty() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/sessions/agent:test:default/messages")
                .header("X-Agent-Ping-Token", "test_token_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_messages_with_pagination() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/sessions/agent:test:default/messages?limit=50&offset=10")
                .header("X-Agent-Ping-Token", "test_token_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_without_token() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_with_wrong_token() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/sessions")
                .header("X-Agent-Ping-Token", "wrong_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_with_correct_token() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/sessions")
                .header("X-Agent-Ping-Token", "test_token_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_whatsapp_inbound_endpoint() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let payload = json!({
        "message": "Hello from WhatsApp",
        "from": "+1234567890"
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/channels/whatsapp/inbound")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_nonexistent_route() {
    let (state, _dir) = create_test_app_state().await;
    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_sessions_with_data() {
    let (state, _dir) = create_test_app_state().await;

    let record = db::SessionRecord {
        session_key: "agent:slack:C123:U456".to_string(),
        agent_id: "test_agent".to_string(),
        business_profile_id: Some("bp_123".to_string()),
        user_id: Some("user_1".to_string()),
        last_route: Some(json!({"channel": "slack"})),
        dm_scope: "per-peer".to_string(),
        identity_links: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    db::upsert_session(&state.pool, state.db_kind, &record).await.unwrap();

    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/sessions")
                .header("X-Agent-Ping-Token", "test_token_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let sessions: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["session_key"], "agent:slack:C123:U456");
}

#[tokio::test]
async fn test_get_session_with_data() {
    let (state, _dir) = create_test_app_state().await;

    let record = db::SessionRecord {
        session_key: "agent:telegram:123456789".to_string(),
        agent_id: "test_agent".to_string(),
        business_profile_id: None,
        user_id: None,
        last_route: Some(json!({"channel": "telegram"})),
        dm_scope: "main".to_string(),
        identity_links: Some(json!({"phone": ["+1234567890"]})),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    db::upsert_session(&state.pool, state.db_kind, &record).await.unwrap();

    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/sessions/agent:telegram:123456789")
                .header("X-Agent-Ping-Token", "test_token_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let session: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(session["session_key"], "agent:telegram:123456789");
    assert_eq!(session["agent_id"], "test_agent");
}

#[tokio::test]
async fn test_list_messages_with_data() {
    let (state, _dir) = create_test_app_state().await;

    let session_key = "agent:whatsapp:biz_123:+1234567890".to_string();
    let record = db::MessageRecord {
        id: "msg_123".to_string(),
        session_key: session_key.clone(),
        direction: "inbound".to_string(),
        channel: "whatsapp".to_string(),
        account_id: Some("biz_123".to_string()),
        peer_id: Some("+1234567890".to_string()),
        content: Some("Test message".to_string()),
        attachments: None,
        status: "received".to_string(),
        dedupe_key: None,
        created_at: chrono::Utc::now(),
    };
    db::insert_message(&state.pool, state.db_kind, &record).await.unwrap();

    let app = create_app(&state);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/v1/sessions/{}/messages", session_key))
                .header("X-Agent-Ping-Token", "test_token_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let messages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["id"], "msg_123");
    assert_eq!(messages[0]["content"], "Test message");
}