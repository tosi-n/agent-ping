pub mod channels;
pub mod config;
pub mod db;
pub mod outbox;
pub mod session;
pub mod types;
pub mod ws;

pub use config::Config;

use self::channels::{slack as slack_channel, telegram as telegram_channel, whatsapp as whatsapp_channel};
use self::config::{load_config, resolve_database_url};
use self::db::DbKind;
use self::types::{Attachment, InboundMessage, OutboundMessage, RouteInfo};

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::AnyPool;
use tokio::sync::{broadcast, mpsc};
use tracing::error;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub pool: AnyPool,
    pub http: reqwest::Client,
    pub ws_tx: broadcast::Sender<ws::WsEvent>,
    pub db_kind: DbKind,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SendMessageRequest {
    pub session_key: String,
    pub text: Option<String>,
    pub attachments: Option<Vec<Attachment>>,
    pub channel: Option<String>,
    pub account_id: Option<String>,
    pub peer_id: Option<String>,
    pub reply_to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub message_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct BulkSendRequest {
    pub messages: Vec<SendMessageRequest>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub sessions: i64,
    pub messages: i64,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn create_app() -> anyhow::Result<(AppState, Router)> {
    sqlx::any::install_default_drivers();

    let config = load_config();
    let db_url = resolve_database_url(&config);
    let db_kind = db::db_kind_from_url(&db_url);
    let pool = AnyPool::connect(&db_url).await?;
    db::init_db(&pool, db_kind).await?;

    let (ws_tx, _) = broadcast::channel(100);
    let state = AppState {
        config: config.clone(),
        pool: pool.clone(),
        http: reqwest::Client::new(),
        ws_tx,
        db_kind,
    };

    let backend_cfg = config.backend.clone();
    tokio::spawn(outbox::start_outbox_worker(pool.clone(), backend_cfg, db_kind));

    if config.channels.telegram.enabled {
        if let Some(token) = config.channels.telegram.bot_token.clone() {
            let (tx, mut rx) = mpsc::channel::<InboundMessage>(100);
            let interval = config.channels.telegram.poll_interval_seconds;
            let state_clone = state.clone();
            tokio::spawn(async move {
                telegram_channel::start_telegram_poller(token, tx, interval).await;
            });
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if let Err(err) = handle_inbound(state_clone.clone(), msg).await {
                        error!("telegram inbound error: {err:?}");
                    }
                }
            });
        }
    }

    let authed_routes = Router::new()
        .route("/v1/messages/send", post(send_message))
        .route("/v1/messages/send-bulk", post(send_bulk))
        .route("/v1/sessions", get(list_sessions))
        .route("/v1/sessions/:session_key", get(get_session))
        .route("/v1/sessions/:session_key/messages", get(list_messages))
        .route("/v1/inbound/ack", post(inbound_ack))
        .route("/v1/ws", get(ws_handler))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let public_routes = Router::new()
        .route("/v1/health", get(health))
        .route("/v1/status", get(status))
        .route(&config.channels.slack.webhook_path, post(slack_events))
        .route(&config.channels.whatsapp.inbound_path, post(whatsapp_inbound));

    let app = Router::new()
        .merge(authed_routes)
        .merge(public_routes)
        .with_state(state.clone());

    Ok((state, app))
}

async fn require_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> impl IntoResponse {
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

async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let sessions = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM sessions")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    let messages = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM messages")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    Json(StatusResponse { sessions, messages })
}

async fn ws_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    let rx = state.ws_tx.subscribe();
    let token = state.config.auth.token.clone();
    ws.on_upgrade(move |socket| ws::handle_ws(socket, rx, token))
}

async fn inbound_ack() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}

async fn send_message(
    State(state): State<AppState>,
    Json(req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let attachments = req.attachments.unwrap_or_default();
    let outbound = OutboundMessage {
        session_key: req.session_key.clone(),
        text: req.text.clone(),
        attachments,
        channel: req.channel.clone(),
        account_id: req.account_id.clone(),
        peer_id: req.peer_id.clone(),
        reply_to: req.reply_to.clone(),
    };

    match handle_outbound(state.clone(), outbound).await {
        Ok(message_id) => Json(SendMessageResponse {
            message_id,
            status: "sent".to_string(),
        })
        .into_response(),
        Err(err) => {
            error!("send_message error: {err:?}");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": err.to_string()})),
            )
                .into_response()
        }
    }
}

async fn send_bulk(State(state): State<AppState>, Json(req): Json<BulkSendRequest>) -> impl IntoResponse {
    let mut results = Vec::new();
    for msg in req.messages {
        let attachments = msg.attachments.unwrap_or_default();
        let outbound = OutboundMessage {
            session_key: msg.session_key.clone(),
            text: msg.text.clone(),
            attachments,
            channel: msg.channel.clone(),
            account_id: msg.account_id.clone(),
            peer_id: msg.peer_id.clone(),
            reply_to: msg.reply_to.clone(),
        };
        match handle_outbound(state.clone(), outbound).await {
            Ok(message_id) => results.push(json!({"message_id": message_id, "status": "sent"})),
            Err(err) => results.push(json!({"error": err.to_string()})),
        }
    }
    Json(json!({"results": results}))
}

async fn list_sessions(State(state): State<AppState>, Query(page): Query<Pagination>) -> impl IntoResponse {
    let limit = page.limit.unwrap_or(100).min(500);
    let offset = page.offset.unwrap_or(0);
    let sessions = db::list_sessions(&state.pool, state.db_kind, limit, offset)
        .await
        .unwrap_or_default();
    Json(sessions)
}

async fn get_session(
    State(state): State<AppState>,
    Path(session_key): Path<String>,
) -> impl IntoResponse {
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
    State(state): State<AppState>,
    Path(session_key): Path<String>,
    Query(page): Query<Pagination>,
) -> impl IntoResponse {
    let limit = page.limit.unwrap_or(200).min(500);
    let offset = page.offset.unwrap_or(0);
    let messages = db::list_messages(&state.pool, state.db_kind, &session_key, limit, offset)
        .await
        .unwrap_or_default();
    Json(messages)
}

async fn slack_events(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    if payload.get("type").and_then(|v| v.as_str()) == Some("url_verification") {
        if let Some(challenge) = payload.get("challenge").and_then(|v| v.as_str()) {
            return Json(json!({"challenge": challenge}));
        }
    }

    if let Some(inbound) = slack_channel::parse_slack_event(&payload) {
        if let Err(err) = handle_inbound(state.clone(), inbound).await {
            error!("slack inbound error: {err:?}");
        }
    }
    Json(json!({"ok": true}))
}

async fn whatsapp_inbound(
    State(state): State<AppState>,
    Json(payload): Json<whatsapp_channel::WhatsAppInboundPayload>,
) -> impl IntoResponse {
    let inbound = whatsapp_channel::normalize_whatsapp_inbound(payload);
    if let Err(err) = handle_inbound(state.clone(), inbound).await {
        error!("whatsapp inbound error: {err:?}");
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": err.to_string()})),
        )
            .into_response();
    }
    Json(json!({"status": "accepted"})).into_response()
}

async fn handle_inbound(state: AppState, mut inbound: InboundMessage) -> anyhow::Result<()> {
    let session_key = session::build_session_key(
        &state.config.session,
        &inbound.channel,
        inbound.account_id.as_deref(),
        &inbound.peer_kind,
        &inbound.peer_id,
        inbound.thread_id.as_deref(),
    );

    let binding = resolve_binding(
        &state.config.bindings,
        &inbound.channel,
        inbound.account_id.as_deref(),
        Some(&inbound.peer_id),
    );
    let agent_id = binding.agent_id.or(Some(state.config.session.agent_id.clone()));

    let last_route = serde_json::json!({
        "channel": inbound.channel,
        "account_id": inbound.account_id,
        "peer_id": inbound.peer_id,
        "thread_id": inbound.thread_id,
    });

    let now = Utc::now();
    let session_record = db::SessionRecord {
        session_key: session_key.clone(),
        agent_id: agent_id.unwrap_or_else(|| "main".to_string()),
        business_profile_id: binding.business_profile_id,
        user_id: binding.user_id,
        last_route: Some(last_route),
        dm_scope: state.config.session.dm_scope.clone(),
        identity_links: if state.config.session.identity_links.is_empty() {
            None
        } else {
            Some(serde_json::to_value(&state.config.session.identity_links).unwrap_or(json!({})))
        },
        created_at: now,
        updated_at: now,
    };
    db::upsert_session(&state.pool, state.db_kind, &session_record).await?;

    if let Some(dedupe_key) = inbound
        .message_id
        .clone()
        .map(|id| format!("{}:{}:{}", inbound.channel, inbound.peer_id, id))
    {
        if db::message_dedupe_exists(&state.pool, state.db_kind, &dedupe_key)
            .await
            .unwrap_or(false)
        {
            return Ok(());
        }
    }

    if !inbound.attachments.is_empty() {
        inbound.attachments =
            upload_media(&state, &inbound.channel, &session_key, &inbound.attachments).await;
    }

    let message_id = uuid::Uuid::new_v4().to_string();
    let dedupe_key = inbound
        .message_id
        .clone()
        .map(|id| format!("{}:{}:{}", inbound.channel, inbound.peer_id, id));

    let record = db::MessageRecord {
        id: message_id.clone(),
        session_key: session_key.clone(),
        direction: "inbound".to_string(),
        channel: inbound.channel.clone(),
        account_id: inbound.account_id.clone(),
        peer_id: Some(inbound.peer_id.clone()),
        content: inbound.text.clone(),
        attachments: Some(serde_json::to_value(&inbound.attachments).unwrap_or(json!([]))),
        status: "received".to_string(),
        dedupe_key,
        created_at: now,
    };
    db::insert_message(&state.pool, state.db_kind, &record).await?;

    let payload = json!({
        "inbound_id": inbound.inbound_id,
        "session_key": session_key,
        "channel": inbound.channel,
        "peer_id": inbound.peer_id,
        "peer_kind": inbound.peer_kind,
        "thread_id": inbound.thread_id,
        "message_id": inbound.message_id,
        "sender_name": inbound.sender_name,
        "text": inbound.text,
        "attachments": inbound.attachments,
        "timestamp": inbound.timestamp,
        "business_profile_id": session_record.business_profile_id,
        "user_id": session_record.user_id,
        "agent_id": session_record.agent_id,
    });

    let next_attempt =
        Utc::now() + chrono::Duration::milliseconds(state.config.queue.debounce_ms as i64);
    let _ = db::insert_outbox(&state.pool, state.db_kind, payload, next_attempt).await?;

    let _ = state.ws_tx.send(ws::WsEvent {
        event: "chat".to_string(),
        payload: json!({"direction": "inbound", "message": record}),
    });

    Ok(())
}

async fn handle_outbound(state: AppState, outbound: OutboundMessage) -> anyhow::Result<String> {
    let session = db::get_session(&state.pool, state.db_kind, &outbound.session_key).await?;
    let route = if let Some(channel) = outbound.channel.clone() {
        RouteInfo {
            channel,
            account_id: outbound.account_id.clone(),
            peer_id: outbound.peer_id.clone(),
            thread_id: None,
        }
    } else if let Some(session) = session {
        if let Some(last_route) = session.last_route {
            let channel = last_route
                .get("channel")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let account_id = last_route
                .get("account_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let peer_id = last_route
                .get("peer_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let thread_id = last_route
                .get("thread_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            RouteInfo {
                channel,
                account_id,
                peer_id,
                thread_id,
            }
        } else {
            return Err(anyhow::anyhow!("no route for session"));
        }
    } else {
        return Err(anyhow::anyhow!("unknown session"));
    };

    let message_id = uuid::Uuid::new_v4().to_string();
    let record = db::MessageRecord {
        id: message_id.clone(),
        session_key: outbound.session_key.clone(),
        direction: "outbound".to_string(),
        channel: route.channel.clone(),
        account_id: route.account_id.clone(),
        peer_id: route.peer_id.clone(),
        content: outbound.text.clone(),
        attachments: Some(serde_json::to_value(&outbound.attachments).unwrap_or(json!([]))),
        status: "queued".to_string(),
        dedupe_key: None,
        created_at: Utc::now(),
    };
    db::insert_message(&state.pool, state.db_kind, &record).await?;

    send_via_channel(&state, &route, &outbound).await?;
    let _ = state.ws_tx.send(ws::WsEvent {
        event: "chat".to_string(),
        payload: json!({"direction": "outbound", "message": record}),
    });

    Ok(message_id)
}

async fn send_via_channel(
    state: &AppState,
    route: &RouteInfo,
    outbound: &OutboundMessage,
) -> anyhow::Result<()> {
    match route.channel.as_str() {
        "slack" => {
            let token = state
                .config
                .channels
                .slack
                .bot_token
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("slack token missing"))?;
            let peer = route
                .peer_id
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("slack peer missing"))?;
            slack_channel::send_slack_message(
                &state.http,
                token,
                peer,
                outbound.text.as_deref(),
                outbound.reply_to.as_deref().or(route.thread_id.as_deref()),
                &outbound.attachments,
            )
            .await?;
        }
        "telegram" => {
            let token = state
                .config
                .channels
                .telegram
                .bot_token
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("telegram token missing"))?;
            let peer = route
                .peer_id
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("telegram peer missing"))?;
            telegram_channel::send_telegram_message(
                &state.http,
                token,
                peer,
                outbound.text.as_deref(),
                outbound.reply_to.as_deref(),
                &outbound.attachments,
            )
            .await?;
        }
        "whatsapp" => {
            let peer = route
                .peer_id
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("whatsapp peer missing"))?;
            whatsapp_channel::send_whatsapp_message(
                &state.http,
                &state.config.channels.whatsapp.sidecar_url,
                peer,
                outbound.text.as_deref(),
                &outbound.attachments,
            )
            .await?;
        }
        _ => return Err(anyhow::anyhow!("unsupported channel")),
    }
    Ok(())
}

async fn upload_media(
    state: &AppState,
    channel: &str,
    session_key: &str,
    attachments: &[Attachment],
) -> Vec<Attachment> {
    let Some(upload_url) = state.config.backend.media_upload_url.as_ref() else {
        return attachments.to_vec();
    };
    let backend_token = state.config.backend.api_token.clone();
    let mut out = Vec::new();

    for att in attachments {
        let mut url = att.url.clone();
        if channel == "telegram" && url.starts_with("telegram://file/") {
            let file_id = url.trim_start_matches("telegram://file/");
            if let Some(token) = state.config.channels.telegram.bot_token.as_ref() {
                if let Ok(Some(real)) =
                    telegram_channel::resolve_telegram_file_url(&state.http, token, file_id).await
                {
                    url = real;
                }
            }
        }

        let mut req = state.http.get(&url);
        if channel == "slack" {
            if let Some(token) = state.config.channels.slack.bot_token.as_ref() {
                req = req.bearer_auth(token);
            }
        }

        let resp = match req.send().await {
            Ok(resp) => resp,
            Err(_) => {
                out.push(att.clone());
                continue;
            }
        };
        let bytes = match resp.bytes().await {
            Ok(bytes) => bytes,
            Err(_) => {
                out.push(att.clone());
                continue;
            }
        };

        let filename = att.filename.clone().unwrap_or_else(|| "file".to_string());
        let part = reqwest::multipart::Part::bytes(bytes.to_vec()).file_name(filename.clone());
        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("channel", channel.to_string())
            .text("session_key", session_key.to_string());
        if let Some(source_id) = &att.id {
            form = form.text("source_id", source_id.to_string());
        }

        let mut upload_req = state.http.post(upload_url).multipart(form);
        if let Some(token) = backend_token.as_ref() {
            upload_req = upload_req.header("X-Agent-Ping-Token", token);
        }

        let uploaded = upload_req.send().await;
        if let Ok(resp) = uploaded {
            if let Ok(value) = resp.json::<serde_json::Value>().await {
                if let Some(storage_url) = value.get("url").and_then(|v| v.as_str()) {
                    out.push(Attachment {
                        id: att.id.clone(),
                        url: storage_url.to_string(),
                        mime_type: att.mime_type.clone(),
                        filename: Some(filename),
                        size: att.size,
                    });
                    continue;
                }
            }
        }
        out.push(att.clone());
    }
    out
}

#[derive(Clone)]
struct BindingMatch {
    business_profile_id: Option<String>,
    user_id: Option<String>,
    agent_id: Option<String>,
}

fn resolve_binding(
    bindings: &[config::Binding],
    channel: &str,
    account_id: Option<&str>,
    peer_id: Option<&str>,
) -> BindingMatch {
    let mut best: Option<(i32, &config::Binding)> = None;
    for binding in bindings {
        if binding.channel != channel {
            continue;
        }
        if let Some(bind_account) = binding.account_id.as_deref() {
            if account_id != Some(bind_account) {
                continue;
            }
        }
        if let Some(bind_peer) = binding.peer_id.as_deref() {
            if peer_id != Some(bind_peer) {
                continue;
            }
        }

        let mut score = 0;
        if binding.account_id.is_some() {
            score += 2;
        }
        if binding.peer_id.is_some() {
            score += 2;
        }
        if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
            best = Some((score, binding));
        }
    }

    if let Some((_, binding)) = best {
        return BindingMatch {
            business_profile_id: binding.business_profile_id.clone(),
            user_id: binding.user_id.clone(),
            agent_id: binding.agent_id.clone(),
        };
    }

    BindingMatch {
        business_profile_id: None,
        user_id: None,
        agent_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Binding;
    use std::collections::HashMap;

    #[test]
    fn test_resolve_binding_no_match() {
        let bindings = vec![Binding {
            channel: "slack".to_string(),
            account_id: Some("ACC1".to_string()),
            peer_id: Some("U1".to_string()),
            business_profile_id: None,
            user_id: None,
            agent_id: None,
        }];
        let result = resolve_binding(&bindings, "telegram", None, Some("U2"));
        assert!(result.agent_id.is_none());
    }

    #[test]
    fn test_resolve_binding_channel_match() {
        let bindings = vec![Binding {
            channel: "slack".to_string(),
            account_id: None,
            peer_id: None,
            business_profile_id: Some("bp_123".to_string()),
            user_id: None,
            agent_id: Some("agent_1".to_string()),
        }];
        let result = resolve_binding(&bindings, "slack", None, None);
        assert_eq!(result.business_profile_id, Some("bp_123".to_string()));
        assert_eq!(result.agent_id, Some("agent_1".to_string()));
    }

    #[test]
    fn test_resolve_binding_account_match() {
        let bindings = vec![Binding {
            channel: "slack".to_string(),
            account_id: Some("ACC123".to_string()),
            peer_id: None,
            business_profile_id: None,
            user_id: Some("user_1".to_string()),
            agent_id: None,
        }];
        let result = resolve_binding(&bindings, "slack", Some("ACC123"), None);
        assert_eq!(result.user_id, Some("user_1".to_string()));
    }

    #[test]
    fn test_resolve_binding_peer_match() {
        let bindings = vec![Binding {
            channel: "whatsapp".to_string(),
            account_id: None,
            peer_id: Some("+1234567890".to_string()),
            business_profile_id: Some("bp_456".to_string()),
            user_id: None,
            agent_id: None,
        }];
        let result = resolve_binding(&bindings, "whatsapp", None, Some("+1234567890"));
        assert_eq!(result.business_profile_id, Some("bp_456".to_string()));
    }

    #[test]
    fn test_resolve_binding_best_score() {
        let bindings = vec![
            Binding {
                channel: "slack".to_string(),
                account_id: None,
                peer_id: Some("U1".to_string()),
                business_profile_id: None,
                user_id: None,
                agent_id: Some("agent_generic".to_string()),
            },
            Binding {
                channel: "slack".to_string(),
                account_id: Some("ACC1".to_string()),
                peer_id: Some("U1".to_string()),
                business_profile_id: None,
                user_id: None,
                agent_id: Some("agent_specific".to_string()),
            },
        ];
        let result = resolve_binding(&bindings, "slack", Some("ACC1"), Some("U1"));
        assert_eq!(result.agent_id, Some("agent_specific".to_string()));
    }

    #[test]
    fn test_send_message_request_default() {
        let req = SendMessageRequest {
            session_key: "test".to_string(),
            text: None,
            attachments: None,
            channel: None,
            account_id: None,
            peer_id: None,
            reply_to: None,
        };
        assert!(req.text.is_none());
        assert!(req.attachments.is_none());
        assert!(req.channel.is_none());
    }

    #[test]
    fn test_bulk_send_request_empty() {
        let req = BulkSendRequest { messages: vec![] };
        assert!(req.messages.is_empty());
    }

    #[test]
    fn test_pagination_limit_offset() {
        let p = Pagination { limit: Some(10), offset: Some(20) };
        assert_eq!(p.limit.unwrap(), 10);
        assert_eq!(p.offset.unwrap(), 20);
    }

    #[test]
    fn test_route_info_partial() {
        let route = RouteInfo {
            channel: "test".to_string(),
            account_id: None,
            peer_id: None,
            thread_id: None,
        };
        assert!(route.account_id.is_none());
        assert!(route.peer_id.is_none());
    }

    #[test]
    fn test_inbound_message_thread() {
        let msg = InboundMessage {
            inbound_id: "in_1".to_string(),
            channel: "slack".to_string(),
            account_id: Some("C123".to_string()),
            peer_id: "U456".to_string(),
            peer_kind: "thread".to_string(),
            thread_id: Some("TS789".to_string()),
            message_id: Some("M123".to_string()),
            sender_name: Some("User".to_string()),
            text: Some("Thread reply".to_string()),
            attachments: vec![],
            timestamp: None,
        };
        assert_eq!(msg.peer_kind, "thread");
        assert_eq!(msg.thread_id, Some("TS789".to_string()));
    }

    #[test]
    fn test_outbound_message_no_reply() {
        let msg = OutboundMessage {
            session_key: "sess_1".to_string(),
            text: Some("New message".to_string()),
            attachments: vec![],
            channel: Some("telegram".to_string()),
            account_id: None,
            peer_id: Some("12345".to_string()),
            reply_to: None,
        };
        assert!(msg.reply_to.is_none());
    }

    #[test]
    fn test_attachment_multiple() {
        let attachments = vec![
            Attachment {
                id: Some("a1".to_string()),
                url: "https://example.com/1.jpg".to_string(),
                mime_type: Some("image/jpeg".to_string()),
                filename: None,
                size: None,
            },
            Attachment {
                id: None,
                url: "https://example.com/2.pdf".to_string(),
                mime_type: Some("application/pdf".to_string()),
                filename: Some("doc.pdf".to_string()),
                size: Some(2048),
            },
        ];
        assert_eq!(attachments.len(), 2);
        assert_eq!(attachments[0].mime_type, Some("image/jpeg".to_string()));
        assert_eq!(attachments[1].size, Some(2048));
    }

    #[test]
    fn test_health_response_variants() {
        let ok = HealthResponse { status: "ok".to_string() };
        let degraded = HealthResponse { status: "degraded".to_string() };
        assert_eq!(ok.status, "ok");
        assert_eq!(degraded.status, "degraded");
    }

    #[test]
    fn test_status_response_counts() {
        let empty = StatusResponse { sessions: 0, messages: 0 };
        let populated = StatusResponse { sessions: 1000, messages: 5000 };
        assert_eq!(empty.sessions, 0);
        assert_eq!(populated.sessions, 1000);
    }

    #[test]
    fn test_send_message_request_with_attachments() {
        let req = SendMessageRequest {
            session_key: "sess_1".to_string(),
            text: Some("Hello".to_string()),
            attachments: Some(vec![Attachment {
                id: Some("a1".to_string()),
                url: "https://example.com/file.jpg".to_string(),
                mime_type: Some("image/jpeg".to_string()),
                filename: Some("file.jpg".to_string()),
                size: Some(1024),
            }]),
            channel: Some("slack".to_string()),
            account_id: Some("C123".to_string()),
            peer_id: Some("U456".to_string()),
            reply_to: None,
        };
        assert!(req.attachments.is_some());
        assert_eq!(req.attachments.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_bulk_send_request_with_messages() {
        let messages = vec![
            SendMessageRequest {
                session_key: "sess_1".to_string(),
                text: Some("Message 1".to_string()),
                attachments: None,
                channel: Some("slack".to_string()),
                account_id: Some("C123".to_string()),
                peer_id: Some("U456".to_string()),
                reply_to: None,
            },
            SendMessageRequest {
                session_key: "sess_2".to_string(),
                text: Some("Message 2".to_string()),
                attachments: None,
                channel: Some("telegram".to_string()),
                account_id: None,
                peer_id: Some("123456789".to_string()),
                reply_to: None,
            },
        ];
        let req = BulkSendRequest { messages: messages.clone() };
        assert_eq!(req.messages.len(), 2);
    }

    #[test]
    fn test_pagination_defaults() {
        let p = Pagination { limit: None, offset: None };
        assert!(p.limit.is_none());
        assert!(p.offset.is_none());
    }

    #[test]
    fn test_route_info_full() {
        let route = RouteInfo {
            channel: "slack".to_string(),
            account_id: Some("C123".to_string()),
            peer_id: Some("U456".to_string()),
            thread_id: Some("TS789".to_string()),
        };
        assert_eq!(route.channel, "slack");
        assert!(route.account_id.is_some());
        assert!(route.peer_id.is_some());
        assert!(route.thread_id.is_some());
    }

    #[test]
    fn test_inbound_message_minimal() {
        let msg = InboundMessage {
            inbound_id: "in_1".to_string(),
            channel: "slack".to_string(),
            account_id: None,
            peer_id: "U456".to_string(),
            peer_kind: "dm".to_string(),
            thread_id: None,
            message_id: None,
            sender_name: None,
            text: None,
            attachments: vec![],
            timestamp: None,
        };
        assert!(msg.account_id.is_none());
        assert!(msg.text.is_none());
        assert!(msg.attachments.is_empty());
    }

    #[test]
    fn test_outbound_message_minimal() {
        let msg = OutboundMessage {
            session_key: "sess_1".to_string(),
            text: None,
            attachments: vec![],
            channel: None,
            account_id: None,
            peer_id: None,
            reply_to: None,
        };
        assert!(msg.text.is_none());
        assert!(msg.channel.is_none());
        assert!(msg.attachments.is_empty());
    }

    #[test]
    fn test_attachment_minimal() {
        let att = Attachment {
            id: None,
            url: "https://example.com/file.txt".to_string(),
            mime_type: None,
            filename: None,
            size: None,
        };
        assert!(att.id.is_none());
        assert!(att.mime_type.is_none());
        assert!(att.filename.is_none());
        assert_eq!(att.url, "https://example.com/file.txt");
    }

    #[test]
    fn test_health_response_degraded() {
        let response = HealthResponse {
            status: "degraded".to_string(),
        };
        assert_eq!(response.status, "degraded");
    }

    #[test]
    fn test_app_state_clone() {
        let config = Config::default();
        assert!(config.server.host.len() > 0);
        assert!(config.server.port > 0);
    }
}