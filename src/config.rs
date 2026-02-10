use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub database: DatabaseConfig,
    pub backend: BackendConfig,
    pub session: SessionConfig,
    pub queue: QueueConfig,
    pub channels: ChannelsConfig,
    pub bindings: Vec<Binding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: Option<String>,
    pub sqlite_path: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: None,
            sqlite_path: "~/.agent-ping/state.sqlite".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub webhook_url: Option<String>,
    pub media_upload_url: Option<String>,
    pub api_token: Option<String>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            webhook_url: None,
            media_upload_url: None,
            api_token: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub agent_id: String,
    pub dm_scope: String,
    pub main_key: String,
    pub identity_links: HashMap<String, Vec<String>>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            agent_id: "main".to_string(),
            dm_scope: "main".to_string(),
            main_key: "main".to_string(),
            identity_links: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub mode: String,
    pub debounce_ms: u64,
    pub cap: usize,
    pub drop: String,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            mode: "collect".to_string(),
            debounce_ms: 1000,
            cap: 20,
            drop: "summarize".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    pub slack: SlackConfig,
    pub telegram: TelegramConfig,
    pub whatsapp: WhatsAppConfig,
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        Self {
            slack: SlackConfig::default(),
            telegram: TelegramConfig::default(),
            whatsapp: WhatsAppConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub enabled: bool,
    pub bot_token: Option<String>,
    pub signing_secret: Option<String>,
    pub app_token: Option<String>,
    pub mode: String,
    pub webhook_path: String,
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bot_token: None,
            signing_secret: None,
            app_token: None,
            mode: "http".to_string(),
            webhook_path: "/v1/channels/slack/events".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: Option<String>,
    pub poll_interval_seconds: u64,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bot_token: None,
            poll_interval_seconds: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    pub enabled: bool,
    pub sidecar_url: String,
    pub inbound_path: String,
}

impl Default for WhatsAppConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sidecar_url: "http://127.0.0.1:4040".to_string(),
            inbound_path: "/v1/channels/whatsapp/inbound".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub channel: String,
    pub account_id: Option<String>,
    pub peer_id: Option<String>,
    pub business_profile_id: Option<String>,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
}

impl Default for Binding {
    fn default() -> Self {
        Self {
            channel: String::new(),
            account_id: None,
            peer_id: None,
            business_profile_id: None,
            user_id: None,
            agent_id: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8091,
            },
            auth: AuthConfig { token: None },
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
                agent_id: "main".to_string(),
                dm_scope: "main".to_string(),
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
                    enabled: false,
                    bot_token: None,
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
            bindings: Vec::new(),
        }
    }
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

pub fn load_config() -> Config {
    let config_path = env::var("AGENT_PING_CONFIG")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| expand_tilde("~/.agent-ping/agent-ping.json"));

    let mut cfg = Config::default();

    if config_path.exists() {
        if let Ok(raw) = fs::read_to_string(&config_path) {
            if let Ok(file_cfg) = serde_json::from_str::<Config>(&raw) {
                cfg = file_cfg;
            }
        }
    }

    // Override from environment
    if let Ok(token) = env::var("AGENT_PING_TOKEN") {
        if !token.trim().is_empty() {
            cfg.auth.token = Some(token);
        }
    }

    if let Ok(url) = env::var("AGENT_PING_DATABASE_URL") {
        if !url.trim().is_empty() {
            cfg.database.url = Some(url);
        }
    }

    if let Ok(path) = env::var("AGENT_PING_SQLITE_PATH") {
        if !path.trim().is_empty() {
            cfg.database.sqlite_path = path;
        }
    }

    if let Ok(url) = env::var("AGENT_PING_BACKEND_WEBHOOK_URL") {
        if !url.trim().is_empty() {
            cfg.backend.webhook_url = Some(url);
        }
    }

    if let Ok(url) = env::var("AGENT_PING_BACKEND_MEDIA_UPLOAD_URL") {
        if !url.trim().is_empty() {
            cfg.backend.media_upload_url = Some(url);
        }
    }

    if let Ok(token) = env::var("AGENT_PING_BACKEND_TOKEN") {
        if !token.trim().is_empty() {
            cfg.backend.api_token = Some(token);
        }
    }

    cfg
}

pub fn resolve_config_path() -> PathBuf {
    env::var("AGENT_PING_CONFIG")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| expand_tilde("~/.agent-ping/agent-ping.json"))
}

pub fn ensure_config_dir() {
    let path = resolve_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

pub fn resolve_database_url(cfg: &Config) -> String {
    if let Some(url) = cfg.database.url.as_ref() {
        return url.to_string();
    }

    let path = expand_tilde(&cfg.database.sqlite_path);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    format!("sqlite://{}", path.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde_with_home() {
        let path = expand_tilde("~/test/file.txt");
        assert!(path.to_string_lossy().contains("test/file.txt"));
    }

    #[test]
    fn test_expand_tilde_no_home() {
        let path = expand_tilde("/absolute/path.txt");
        assert_eq!(path, PathBuf::from("/absolute/path.txt"));
    }

    #[test]
    fn test_expand_tilde_empty() {
        let path = expand_tilde("");
        assert_eq!(path, PathBuf::from(""));
    }

    #[test]
    fn test_resolve_database_url_with_url() {
        let cfg = Config {
            database: DatabaseConfig {
                url: Some("postgres://localhost/testdb".to_string()),
                sqlite_path: "~/.agent-ping/state.sqlite".to_string(),
            },
            ..Config::default()
        };
        let url = resolve_database_url(&cfg);
        assert_eq!(url, "postgres://localhost/testdb");
    }

    #[test]
    fn test_resolve_database_url_without_url() {
        let cfg = Config {
            database: DatabaseConfig {
                url: None,
                sqlite_path: "~/test/data.db".to_string(),
            },
            ..Config::default()
        };
        let url = resolve_database_url(&cfg);
        assert!(url.starts_with("sqlite://"));
    }

    #[test]
    fn test_resolve_config_path_default() {
        std::env::remove_var("AGENT_PING_CONFIG");
        let path = resolve_config_path();
        assert!(
            path.ends_with("agent-ping.json") || path.to_string_lossy().contains(".agent-ping")
        );
    }

    #[test]
    fn test_resolve_config_path_env_override() {
        std::env::set_var("AGENT_PING_CONFIG", "/custom/path/config.json");
        let path = resolve_config_path();
        assert_eq!(path, PathBuf::from("/custom/path/config.json"));
        std::env::remove_var("AGENT_PING_CONFIG");
    }

    #[test]
    fn test_config_default_values() {
        let cfg = Config::default();
        assert_eq!(cfg.server.port, 8091);
        assert_eq!(cfg.server.host, "0.0.0.0");
        assert!(cfg.auth.token.is_none());
        assert!(cfg.bindings.is_empty());
    }

    #[test]
    fn test_session_config_default() {
        let session = SessionConfig::default();
        assert_eq!(session.agent_id, "main");
        assert_eq!(session.dm_scope, "main");
        assert_eq!(session.main_key, "main");
        assert!(session.identity_links.is_empty());
    }

    #[test]
    fn test_channels_config_default() {
        let channels = ChannelsConfig::default();
        assert!(!channels.slack.enabled);
        assert!(!channels.telegram.enabled);
        assert!(!channels.whatsapp.enabled);
        assert_eq!(channels.whatsapp.sidecar_url, "http://127.0.0.1:4040");
    }

    #[test]
    fn test_queue_config_default() {
        let queue = QueueConfig::default();
        assert_eq!(queue.mode, "collect");
        assert_eq!(queue.debounce_ms, 1000);
        assert_eq!(queue.cap, 20);
        assert_eq!(queue.drop, "summarize");
    }

    #[test]
    fn test_backend_config_default() {
        let backend = BackendConfig::default();
        assert!(backend.webhook_url.is_none());
        assert!(backend.media_upload_url.is_none());
        assert!(backend.api_token.is_none());
    }

    #[test]
    fn test_database_config_default() {
        let db = DatabaseConfig::default();
        assert!(db.url.is_none());
        assert_eq!(db.sqlite_path, "~/.agent-ping/state.sqlite");
    }

    #[test]
    fn test_slack_config_default() {
        let slack = SlackConfig::default();
        assert!(!slack.enabled);
        assert!(slack.bot_token.is_none());
        assert_eq!(slack.webhook_path, "/v1/channels/slack/events");
    }

    #[test]
    fn test_telegram_config_default() {
        let tg = TelegramConfig::default();
        assert!(!tg.enabled);
        assert!(tg.bot_token.is_none());
        assert_eq!(tg.poll_interval_seconds, 2);
    }

    #[test]
    fn test_whatsapp_config_default() {
        let wa = WhatsAppConfig::default();
        assert!(!wa.enabled);
        assert_eq!(wa.sidecar_url, "http://127.0.0.1:4040");
        assert_eq!(wa.inbound_path, "/v1/channels/whatsapp/inbound");
    }

    #[test]
    fn test_binding_default() {
        let binding = Binding::default();
        assert!(binding.channel.is_empty());
        assert!(binding.account_id.is_none());
        assert!(binding.peer_id.is_none());
        assert!(binding.business_profile_id.is_none());
        assert!(binding.user_id.is_none());
        assert!(binding.agent_id.is_none());
    }
}
