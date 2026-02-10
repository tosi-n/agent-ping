use agent_ping::config::{
    AuthConfig, BackendConfig, Binding, ChannelsConfig, Config, DatabaseConfig, QueueConfig,
    ServerConfig, SessionConfig, SlackConfig, TelegramConfig, WhatsAppConfig,
};
use agent_ping::db::DbKind;
use agent_ping::types::{Attachment, InboundMessage, OutboundMessage, RouteInfo};
use std::collections::HashMap;

#[test]
fn test_inbound_message_structure() {
    let msg = InboundMessage {
        inbound_id: "in_123".to_string(),
        channel: "slack".to_string(),
        account_id: Some("ACC123".to_string()),
        peer_id: "U456".to_string(),
        peer_kind: "dm".to_string(),
        thread_id: Some("TS123".to_string()),
        message_id: Some("MSG456".to_string()),
        sender_name: Some("John Doe".to_string()),
        text: Some("Hello".to_string()),
        attachments: vec![Attachment {
            id: Some("att1".to_string()),
            url: "https://example.com/file.jpg".to_string(),
            mime_type: Some("image/jpeg".to_string()),
            filename: Some("file.jpg".to_string()),
            size: Some(1024),
        }],
        timestamp: Some("1234567890".to_string()),
    };

    assert_eq!(msg.channel, "slack");
    assert_eq!(msg.peer_kind, "dm");
    assert_eq!(msg.attachments.len(), 1);
    assert_eq!(msg.attachments[0].mime_type, Some("image/jpeg".to_string()));
}

#[test]
fn test_outbound_message_structure() {
    let msg = OutboundMessage {
        session_key: "agent:test:default".to_string(),
        text: Some("Hello from bot".to_string()),
        attachments: vec![Attachment {
            id: None,
            url: "https://example.com/image.png".to_string(),
            mime_type: Some("image/png".to_string()),
            filename: None,
            size: None,
        }],
        channel: Some("slack".to_string()),
        account_id: Some("ACC123".to_string()),
        peer_id: Some("U456".to_string()),
        reply_to: Some("MSG789".to_string()),
    };

    assert_eq!(msg.session_key, "agent:test:default");
    assert_eq!(msg.channel, Some("slack".to_string()));
    assert_eq!(msg.reply_to, Some("MSG789".to_string()));
    assert_eq!(msg.attachments.len(), 1);
}

#[test]
fn test_attachment_variants() {
    let att1 = Attachment {
        id: Some("id1".to_string()),
        url: "https://example.com/file1.pdf".to_string(),
        mime_type: Some("application/pdf".to_string()),
        filename: Some("document.pdf".to_string()),
        size: Some(2048),
    };

    let att2 = Attachment {
        id: None,
        url: "https://example.com/file2.txt".to_string(),
        mime_type: None,
        filename: None,
        size: None,
    };

    assert!(att1.id.is_some());
    assert!(att2.id.is_none());
    assert!(att2.mime_type.is_none());
}

#[test]
fn test_db_kind_from_url() {
    assert_eq!(DbKind::Sqlite, db_kind_from_url("sqlite://test.db"));
    assert_eq!(DbKind::Sqlite, db_kind_from_url("SQLite://test.db"));

    assert_eq!(
        DbKind::Postgres,
        db_kind_from_url("postgres://localhost/testdb")
    );
    assert_eq!(
        DbKind::Postgres,
        db_kind_from_url("postgresql://localhost/testdb")
    );
}

#[test]
fn test_rewrite_sql_sqlite() {
    let sql = "SELECT * FROM test WHERE id = ? AND name = ?";
    let rewritten = rewrite_sql(sql, DbKind::Sqlite);
    assert_eq!(rewritten.as_ref(), sql);
}

#[test]
fn test_rewrite_sql_postgres() {
    let sql = "SELECT * FROM test WHERE id = ? AND name = ?";
    let rewritten = rewrite_sql(sql, DbKind::Postgres);
    assert_eq!(
        rewritten.as_ref(),
        "SELECT * FROM test WHERE id = $1 AND name = $2"
    );
}

#[test]
fn test_rewrite_sql_complex() {
    let sql = "SELECT a.*, b.name FROM table_a a JOIN table_b b ON a.id = b.a_id WHERE a.x = ? AND b.y = ? AND c.z = ?";
    let rewritten = rewrite_sql(sql, DbKind::Postgres);
    assert_eq!(rewritten.as_ref(), "SELECT a.*, b.name FROM table_a a JOIN table_b b ON a.id = b.a_id WHERE a.x = $1 AND b.y = $2 AND c.z = $3");
}

#[test]
fn test_rewrite_sql_no_placeholders() {
    let sql = "SELECT COUNT(*) FROM users";
    let rewritten = rewrite_sql(sql, DbKind::Postgres);
    assert_eq!(rewritten.as_ref(), sql);
}

fn db_kind_from_url(url: &str) -> DbKind {
    let lower = url.to_lowercase();
    if lower.starts_with("postgres://") || lower.starts_with("postgresql://") {
        DbKind::Postgres
    } else {
        DbKind::Sqlite
    }
}

fn rewrite_sql<'a>(sql: &'a str, kind: DbKind) -> std::borrow::Cow<'a, str> {
    match kind {
        DbKind::Sqlite => std::borrow::Cow::Borrowed(sql),
        DbKind::Postgres => {
            let mut out = String::with_capacity(sql.len() + 8);
            let mut idx = 1;
            for ch in sql.chars() {
                if ch == '?' {
                    out.push('$');
                    out.push_str(&idx.to_string());
                    idx += 1;
                } else {
                    out.push(ch);
                }
            }
            std::borrow::Cow::Owned(out)
        }
    }
}

#[test]
fn test_config_server_listen() {
    let config = Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
        },
        ..Config::default()
    };

    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 3000);
}

#[test]
fn test_inbound_message_slack_dm() {
    let inbound = InboundMessage {
        inbound_id: "in_123".to_string(),
        channel: "slack".to_string(),
        account_id: Some("C123".to_string()),
        peer_id: "U456".to_string(),
        peer_kind: "dm".to_string(),
        thread_id: None,
        message_id: Some("M123".to_string()),
        sender_name: Some("Test User".to_string()),
        text: Some("Hello".to_string()),
        attachments: vec![],
        timestamp: Some("1699999999".to_string()),
    };

    assert_eq!(inbound.channel, "slack");
    assert_eq!(inbound.peer_id, "U456");
    assert_eq!(inbound.peer_kind, "dm");
    assert!(inbound.text.is_some());
}

#[test]
fn test_inbound_message_telegram() {
    let inbound = InboundMessage {
        inbound_id: "in_tg_123".to_string(),
        channel: "telegram".to_string(),
        account_id: None,
        peer_id: "123456789".to_string(),
        peer_kind: "private".to_string(),
        thread_id: None,
        message_id: Some("tg_msg_456".to_string()),
        sender_name: Some("Telegram User".to_string()),
        text: Some("Hello from Telegram".to_string()),
        attachments: vec![],
        timestamp: Some("1700000000".to_string()),
    };

    assert_eq!(inbound.channel, "telegram");
    assert!(inbound.account_id.is_none());
    assert_eq!(inbound.peer_kind, "private");
}

#[test]
fn test_inbound_message_whatsapp() {
    let inbound = InboundMessage {
        inbound_id: "in_wa_123".to_string(),
        channel: "whatsapp".to_string(),
        account_id: Some("business_123".to_string()),
        peer_id: "+1234567890".to_string(),
        peer_kind: "private".to_string(),
        thread_id: None,
        message_id: Some("wa_msg_789".to_string()),
        sender_name: Some("WhatsApp Contact".to_string()),
        text: Some("Hello from WhatsApp".to_string()),
        attachments: vec![],
        timestamp: None,
    };

    assert_eq!(inbound.channel, "whatsapp");
    assert_eq!(inbound.account_id, Some("business_123".to_string()));
}

#[test]
fn test_inbound_message_with_thread() {
    let inbound = InboundMessage {
        inbound_id: "in_thread_123".to_string(),
        channel: "slack".to_string(),
        account_id: Some("C123".to_string()),
        peer_id: "U456".to_string(),
        peer_kind: "thread".to_string(),
        thread_id: Some("thread_ts_123".to_string()),
        message_id: Some("M123".to_string()),
        sender_name: Some("Thread User".to_string()),
        text: Some("Thread reply".to_string()),
        attachments: vec![],
        timestamp: None,
    };

    assert_eq!(inbound.peer_kind, "thread");
    assert_eq!(inbound.thread_id, Some("thread_ts_123".to_string()));
}

#[test]
fn test_inbound_message_empty() {
    let inbound = InboundMessage {
        inbound_id: "in_empty".to_string(),
        channel: "telegram".to_string(),
        account_id: None,
        peer_id: "987654321".to_string(),
        peer_kind: "private".to_string(),
        thread_id: None,
        message_id: None,
        sender_name: None,
        text: None,
        attachments: vec![],
        timestamp: None,
    };

    assert!(inbound.text.is_none());
    assert!(inbound.attachments.is_empty());
    assert!(inbound.message_id.is_none());
}

#[test]
fn test_outbound_message_basic() {
    let outbound = OutboundMessage {
        session_key: "agent:test:default".to_string(),
        text: Some("Hello from bot".to_string()),
        attachments: vec![],
        channel: Some("slack".to_string()),
        account_id: Some("C123".to_string()),
        peer_id: Some("U456".to_string()),
        reply_to: None,
    };

    assert_eq!(outbound.session_key, "agent:test:default");
    assert_eq!(outbound.channel, Some("slack".to_string()));
    assert!(outbound.reply_to.is_none());
}

#[test]
fn test_outbound_message_with_reply() {
    let outbound = OutboundMessage {
        session_key: "agent:test:default".to_string(),
        text: Some("Reply message".to_string()),
        attachments: vec![],
        channel: Some("slack".to_string()),
        account_id: Some("C123".to_string()),
        peer_id: Some("U456".to_string()),
        reply_to: Some("original_msg_id".to_string()),
    };

    assert_eq!(outbound.reply_to, Some("original_msg_id".to_string()));
}

#[test]
fn test_outbound_message_telegram() {
    let outbound = OutboundMessage {
        session_key: "agent:telegram:default".to_string(),
        text: Some("Hello Telegram".to_string()),
        attachments: vec![],
        channel: Some("telegram".to_string()),
        account_id: None,
        peer_id: Some("123456789".to_string()),
        reply_to: None,
    };

    assert_eq!(outbound.channel, Some("telegram".to_string()));
    assert_eq!(outbound.session_key, "agent:telegram:default");
}

#[test]
fn test_route_info_slack() {
    let route = RouteInfo {
        channel: "slack".to_string(),
        account_id: Some("C123".to_string()),
        peer_id: Some("U456".to_string()),
        thread_id: Some("TS789".to_string()),
    };

    assert_eq!(route.channel, "slack");
    assert_eq!(route.account_id, Some("C123".to_string()));
    assert_eq!(route.thread_id, Some("TS789".to_string()));
}

#[test]
fn test_route_info_telegram() {
    let route = RouteInfo {
        channel: "telegram".to_string(),
        account_id: None,
        peer_id: Some("123456789".to_string()),
        thread_id: None,
    };

    assert_eq!(route.channel, "telegram");
    assert!(route.account_id.is_none());
    assert!(route.thread_id.is_none());
}

#[test]
fn test_route_info_whatsapp() {
    let route = RouteInfo {
        channel: "whatsapp".to_string(),
        account_id: Some("business_123".to_string()),
        peer_id: Some("+1234567890".to_string()),
        thread_id: None,
    };

    assert_eq!(route.channel, "whatsapp");
    assert_eq!(route.account_id, Some("business_123".to_string()));
}

#[test]
fn test_attachment_image() {
    let att = Attachment {
        id: Some("img_123".to_string()),
        url: "https://example.com/image.jpg".to_string(),
        mime_type: Some("image/jpeg".to_string()),
        filename: Some("photo.jpg".to_string()),
        size: Some(102400),
    };

    assert_eq!(att.id, Some("img_123".to_string()));
    assert_eq!(att.mime_type, Some("image/jpeg".to_string()));
    assert_eq!(att.size, Some(102400));
}

#[test]
fn test_attachment_document() {
    let att = Attachment {
        id: None,
        url: "https://example.com/document.pdf".to_string(),
        mime_type: Some("application/pdf".to_string()),
        filename: Some("report.pdf".to_string()),
        size: Some(2048000),
    };

    assert!(att.id.is_none());
    assert_eq!(att.mime_type, Some("application/pdf".to_string()));
    assert_eq!(att.filename, Some("report.pdf".to_string()));
}

#[test]
fn test_config_with_bindings() {
    let bindings = vec![Binding {
        channel: "slack".to_string(),
        account_id: Some("C123".to_string()),
        peer_id: None,
        business_profile_id: Some("bp_123".to_string()),
        user_id: None,
        agent_id: None,
    }];

    let config = Config {
        bindings,
        ..Config::default()
    };

    assert_eq!(config.bindings.len(), 1);
    assert_eq!(config.bindings[0].channel, "slack");
    assert_eq!(
        config.bindings[0].business_profile_id,
        Some("bp_123".to_string())
    );
}

#[test]
fn test_config_multiple_bindings() {
    let bindings = vec![
        Binding {
            channel: "slack".to_string(),
            account_id: Some("C1".to_string()),
            peer_id: Some("U1".to_string()),
            business_profile_id: None,
            user_id: None,
            agent_id: Some("agent_1".to_string()),
        },
        Binding {
            channel: "telegram".to_string(),
            account_id: None,
            peer_id: Some("tg_123".to_string()),
            business_profile_id: None,
            user_id: None,
            agent_id: None,
        },
    ];

    let config = Config {
        bindings,
        ..Config::default()
    };

    assert_eq!(config.bindings.len(), 2);
    assert_eq!(config.bindings[0].agent_id, Some("agent_1".to_string()));
    assert_eq!(config.bindings[1].channel, "telegram");
}

#[test]
fn test_config_telegram_enabled() {
    let config = Config {
        channels: ChannelsConfig {
            telegram: TelegramConfig {
                enabled: true,
                bot_token: Some("bot_token_123".to_string()),
                poll_interval_seconds: 5,
            },
            ..ChannelsConfig::default()
        },
        ..Config::default()
    };

    assert!(config.channels.telegram.enabled);
    assert_eq!(
        config.channels.telegram.bot_token,
        Some("bot_token_123".to_string())
    );
    assert_eq!(config.channels.telegram.poll_interval_seconds, 5);
}

#[test]
fn test_config_whatsapp_enabled() {
    let config = Config {
        channels: ChannelsConfig {
            whatsapp: WhatsAppConfig {
                enabled: true,
                sidecar_url: "http://whatsapp:4040".to_string(),
                inbound_path: "/v1/whatsapp".to_string(),
            },
            ..ChannelsConfig::default()
        },
        ..Config::default()
    };

    assert!(config.channels.whatsapp.enabled);
    assert_eq!(config.channels.whatsapp.sidecar_url, "http://whatsapp:4040");
}

#[test]
fn test_config_backend_settings() {
    let config = Config {
        backend: BackendConfig {
            webhook_url: Some("https://backend.example.com/webhook".to_string()),
            media_upload_url: Some("https://backend.example.com/upload".to_string()),
            api_token: Some("secret_token".to_string()),
        },
        ..Config::default()
    };

    assert_eq!(
        config.backend.media_upload_url,
        Some("https://backend.example.com/upload".to_string())
    );
    assert_eq!(config.backend.api_token, Some("secret_token".to_string()));
}

#[test]
fn test_config_session_identity_links() {
    let mut identity_links = HashMap::new();
    identity_links.insert("email".to_string(), vec!["user@example.com".to_string()]);
    identity_links.insert("phone".to_string(), vec!["+1234567890".to_string()]);
    identity_links.insert(
        "slack".to_string(),
        vec!["U123".to_string(), "U456".to_string()],
    );

    let config = Config {
        session: SessionConfig {
            agent_id: "test_agent".to_string(),
            dm_scope: "per-peer".to_string(),
            main_key: "main".to_string(),
            identity_links,
            ..SessionConfig::default()
        },
        ..Config::default()
    };

    assert_eq!(config.session.identity_links.len(), 3);
    assert_eq!(
        config.session.identity_links["email"],
        vec!["user@example.com"]
    );
    assert_eq!(config.session.dm_scope, "per-peer");
}

#[test]
fn test_config_queue_settings() {
    let config = Config {
        queue: QueueConfig {
            mode: "immediate".to_string(),
            debounce_ms: 50,
            cap: 10,
            drop: "error".to_string(),
        },
        ..Config::default()
    };

    assert_eq!(config.queue.mode, "immediate");
    assert_eq!(config.queue.debounce_ms, 50);
    assert_eq!(config.queue.cap, 10);
}

#[test]
fn test_empty_outbound_message() {
    let outbound = OutboundMessage {
        session_key: "agent:test:default".to_string(),
        text: None,
        attachments: vec![],
        channel: None,
        account_id: None,
        peer_id: None,
        reply_to: None,
    };

    assert!(outbound.text.is_none());
    assert!(outbound.channel.is_none());
    assert!(outbound.attachments.is_empty());
}

#[test]
fn test_attachment_empty_optional_fields() {
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
    assert!(att.size.is_none());
}
