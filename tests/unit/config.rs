use agent_ping::config::{
    expand_tilde, load_config, resolve_config_path, resolve_database_url, Config,
};
use std::collections::HashMap;

#[test]
fn test_default_config() {
    let cfg = Config::default();
    assert_eq!(cfg.server.host, "0.0.0.0");
    assert_eq!(cfg.server.port, 8091);
    assert_eq!(cfg.session.agent_id, "main");
    assert_eq!(cfg.session.dm_scope, "main");
    assert_eq!(cfg.session.main_key, "main");
    assert!(!cfg.channels.slack.enabled);
    assert!(!cfg.channels.telegram.enabled);
    assert!(!cfg.channels.whatsapp.enabled);
    assert!(cfg.auth.token.is_none());
    assert_eq!(cfg.queue.debounce_ms, 1000);
    assert_eq!(cfg.queue.cap, 20);
}

#[test]
fn test_default_server_config() {
    let cfg = Config::default();
    assert_eq!(cfg.server.host, "0.0.0.0");
    assert_eq!(cfg.server.port, 8091);
}

#[test]
fn test_default_auth_config() {
    let cfg = Config::default();
    assert!(cfg.auth.token.is_none());
}

#[test]
fn test_default_database_config() {
    let cfg = Config::default();
    assert!(cfg.database.url.is_none());
    assert_eq!(cfg.database.sqlite_path, "~/.agent-ping/state.sqlite");
}

#[test]
fn test_default_session_config() {
    let cfg = Config::default();
    assert_eq!(cfg.session.agent_id, "main");
    assert_eq!(cfg.session.dm_scope, "main");
    assert_eq!(cfg.session.main_key, "main");
    assert!(cfg.session.identity_links.is_empty());
}

#[test]
fn test_default_queue_config() {
    let cfg = Config::default();
    assert_eq!(cfg.queue.mode, "collect");
    assert_eq!(cfg.queue.debounce_ms, 1000);
    assert_eq!(cfg.queue.cap, 20);
    assert_eq!(cfg.queue.drop, "summarize");
}

#[test]
fn test_default_channels_config() {
    let cfg = Config::default();
    assert!(!cfg.channels.slack.enabled);
    assert!(cfg.channels.slack.bot_token.is_none());
    assert_eq!(cfg.channels.slack.webhook_path, "/v1/channels/slack/events");

    assert!(!cfg.channels.telegram.enabled);
    assert!(cfg.channels.telegram.bot_token.is_none());
    assert_eq!(cfg.channels.telegram.poll_interval_seconds, 2);

    assert!(!cfg.channels.whatsapp.enabled);
    assert_eq!(cfg.channels.whatsapp.sidecar_url, "http://127.0.0.1:4040");
    assert_eq!(
        cfg.channels.whatsapp.inbound_path,
        "/v1/channels/whatsapp/inbound"
    );
}

#[test]
fn test_default_bindings() {
    let cfg = Config::default();
    assert!(cfg.bindings.is_empty());
}

#[test]
fn test_expand_tilde() {
    let expanded = expand_tilde("~/test/path");
    assert!(expanded.to_string_lossy().contains("test/path"));
}

#[test]
fn test_expand_tilde_no_tilde() {
    let expanded = expand_tilde("/absolute/path");
    assert_eq!(expanded.to_string_lossy(), "/absolute/path");
}

#[test]
fn test_resolve_config_path_default() {
    std::env::remove_var("AGENT_PING_CONFIG");
    let path = resolve_config_path();
    assert!(path.to_string_lossy().contains(".agent-ping"));
    assert!(path.to_string_lossy().contains("agent-ping.json"));
}

#[test]
fn test_resolve_database_url_sqlite() {
    let cfg = Config::default();
    let url = resolve_database_url(&cfg);
    assert!(url.starts_with("sqlite://"));
}

#[test]
fn test_resolve_database_url_postgres() {
    let mut cfg = Config::default();
    cfg.database.url = Some("postgres://localhost/testdb".to_string());
    let url = resolve_database_url(&cfg);
    assert_eq!(url, "postgres://localhost/testdb");
}
