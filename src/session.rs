use crate::config::SessionConfig;

pub fn normalize_token(value: &str) -> String {
    value.trim().to_lowercase()
}

pub fn resolve_identity_link(
    links: &std::collections::HashMap<String, Vec<String>>,
    channel: &str,
    peer_id: &str,
) -> Option<String> {
    let peer_norm = normalize_token(peer_id);
    if peer_norm.is_empty() {
        return None;
    }
    let channel_norm = normalize_token(channel);
    let scoped = if channel_norm.is_empty() {
        peer_norm.clone()
    } else {
        format!("{}:{}", channel_norm, peer_norm)
    };
    for (canonical, values) in links {
        for value in values {
            let v = normalize_token(value);
            if v == peer_norm || v == scoped {
                return Some(normalize_token(canonical));
            }
        }
    }
    None
}

pub fn build_session_key(
    cfg: &SessionConfig,
    channel: &str,
    account_id: Option<&str>,
    peer_kind: &str,
    peer_id: &str,
    thread_id: Option<&str>,
) -> String {
    let agent_id = cfg.agent_id.trim().to_lowercase();
    let main_key = cfg.main_key.trim().to_lowercase();
    let channel = normalize_token(channel);
    let peer_id = normalize_token(peer_id);
    let account_id = account_id
        .map(normalize_token)
        .unwrap_or_else(|| "default".to_string());
    let dm_scope = cfg.dm_scope.as_str();

    if peer_kind == "dm" {
        let mut key_peer = peer_id.clone();
        if dm_scope != "main" && !cfg.identity_links.is_empty() {
            if let Some(canonical) = resolve_identity_link(&cfg.identity_links, &channel, &peer_id)
            {
                key_peer = canonical;
            }
        }
        return match dm_scope {
            "per-peer" => format!("agent:{}:dm:{}", agent_id, key_peer),
            "per-channel-peer" => format!("agent:{}:{}:dm:{}", agent_id, channel, key_peer),
            "per-account-channel-peer" => {
                format!(
                    "agent:{}:{}:{}:dm:{}",
                    agent_id, channel, account_id, key_peer
                )
            }
            _ => format!("agent:{}:{}", agent_id, main_key),
        };
    }

    let mut base = format!("agent:{}:{}:{}:{}", agent_id, channel, peer_kind, peer_id);
    if let Some(thread) = thread_id {
        let thread = normalize_token(thread);
        if !thread.is_empty() {
            base = format!("{}:thread:{}", base, thread);
        }
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_normalize_token() {
        assert_eq!(normalize_token("Hello"), "hello");
        assert_eq!(normalize_token("  World  "), "world");
        assert_eq!(normalize_token("mixedCASE"), "mixedcase");
    }

    #[test]
    fn test_normalize_token_empty() {
        assert_eq!(normalize_token("   "), "");
    }

    #[test]
    fn test_resolve_identity_link_empty_peer() {
        let links: HashMap<String, Vec<String>> = HashMap::new();
        let result = resolve_identity_link(&links, "slack", "");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_identity_link_no_match() {
        let mut links = HashMap::new();
        links.insert("email".to_string(), vec!["user@example.com".to_string()]);
        let result = resolve_identity_link(&links, "slack", "unknown_user");
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_identity_link_channel_peer_scoped() {
        let mut links = HashMap::new();
        links.insert("canonical".to_string(), vec!["slack:u123".to_string()]);
        let result = resolve_identity_link(&links, "slack", "u123");
        assert_eq!(result, Some("canonical".to_string()));
    }

    #[test]
    fn test_build_session_key_dm_per_peer() {
        let cfg = SessionConfig {
            agent_id: "myagent".to_string(),
            dm_scope: "per-peer".to_string(),
            main_key: "main".to_string(),
            identity_links: HashMap::new(),
        };
        let key = build_session_key(&cfg, "slack", None, "dm", "U123", None);
        assert_eq!(key, "agent:myagent:dm:u123");
    }

    #[test]
    fn test_build_session_key_dm_per_channel_peer() {
        let cfg = SessionConfig {
            agent_id: "myagent".to_string(),
            dm_scope: "per-channel-peer".to_string(),
            main_key: "main".to_string(),
            identity_links: HashMap::new(),
        };
        let key = build_session_key(&cfg, "telegram", None, "dm", "tg456", None);
        assert_eq!(key, "agent:myagent:telegram:dm:tg456");
    }

    #[test]
    fn test_build_session_key_dm_per_account_channel_peer() {
        let cfg = SessionConfig {
            agent_id: "myagent".to_string(),
            dm_scope: "per-account-channel-peer".to_string(),
            main_key: "main".to_string(),
            identity_links: HashMap::new(),
        };
        let key = build_session_key(&cfg, "whatsapp", Some("biz123"), "dm", "wa789", None);
        assert_eq!(key, "agent:myagent:whatsapp:biz123:dm:wa789");
    }

    #[test]
    fn test_build_session_key_dm_main() {
        let cfg = SessionConfig {
            agent_id: "myagent".to_string(),
            dm_scope: "main".to_string(),
            main_key: "main".to_string(),
            identity_links: HashMap::new(),
        };
        let key = build_session_key(&cfg, "slack", None, "dm", "U123", None);
        assert_eq!(key, "agent:myagent:main");
    }

    #[test]
    fn test_build_session_key_thread() {
        let cfg = SessionConfig {
            agent_id: "myagent".to_string(),
            dm_scope: "per-peer".to_string(),
            main_key: "main".to_string(),
            identity_links: HashMap::new(),
        };
        let key = build_session_key(&cfg, "slack", Some("C123"), "thread", "U456", Some("ts789"));
        assert_eq!(key, "agent:myagent:slack:thread:u456:thread:ts789");
    }

    #[test]
    fn test_build_session_key_empty_thread() {
        let cfg = SessionConfig {
            agent_id: "myagent".to_string(),
            dm_scope: "per-peer".to_string(),
            main_key: "main".to_string(),
            identity_links: HashMap::new(),
        };
        let key = build_session_key(&cfg, "slack", None, "thread", "U456", Some("   "));
        assert_eq!(key, "agent:myagent:slack:thread:u456");
    }

    #[test]
    fn test_build_session_key_whitespace_normalized() {
        let cfg = SessionConfig {
            agent_id: "MyAgent".to_string(),
            dm_scope: "per-peer".to_string(),
            main_key: "Main".to_string(),
            identity_links: HashMap::new(),
        };
        let key = build_session_key(&cfg, "  Slack  ", Some("  C123  "), "dm", "  U456  ", None);
        assert_eq!(key, "agent:myagent:dm:u456");
    }
}
