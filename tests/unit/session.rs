use agent_ping::config::SessionConfig;
use agent_ping::session::{build_session_key, resolve_identity_link};
use std::collections::HashMap;

#[test]
fn test_dm_main_scope() {
    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "main".to_string(),
        main_key: "default".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "slack", None, "dm", "user123", None);
    assert_eq!(key, "agent:myagent:default");
}

#[test]
fn test_dm_per_peer_scope() {
    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "per-peer".to_string(),
        main_key: "default".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "slack", None, "dm", "user123", None);
    assert_eq!(key, "agent:myagent:dm:user123");
}

#[test]
fn test_dm_per_channel_peer_scope() {
    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "per-channel-peer".to_string(),
        main_key: "default".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "slack", None, "dm", "user123", None);
    assert_eq!(key, "agent:myagent:slack:dm:user123");
}

#[test]
fn test_dm_per_account_channel_peer_scope() {
    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "per-account-channel-peer".to_string(),
        main_key: "default".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "slack", Some("account1"), "dm", "user123", None);
    assert_eq!(key, "agent:myagent:slack:account1:dm:user123");
}

#[test]
fn test_channel_with_thread() {
    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "main".to_string(),
        main_key: "default".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "slack", None, "channel", "C123", Some("thread1"));
    assert_eq!(key, "agent:myagent:slack:channel:c123:thread:thread1");
}

#[test]
fn test_channel_without_thread() {
    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "main".to_string(),
        main_key: "default".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "telegram", None, "channel", "C456", None);
    assert_eq!(key, "agent:myagent:telegram:channel:c456");
}

#[test]
fn test_group_peer_kind() {
    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "main".to_string(),
        main_key: "default".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "telegram", None, "group", "-123456", None);
    assert_eq!(key, "agent:myagent:telegram:group:-123456");
}

#[test]
fn test_empty_thread_id_is_ignored() {
    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "main".to_string(),
        main_key: "default".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "slack", None, "channel", "C123", Some(""));
    assert_eq!(key, "agent:myagent:slack:channel:c123");
}

#[test]
fn test_case_normalization() {
    let cfg = SessionConfig {
        agent_id: "MyAgent".to_string(),
        dm_scope: "Main".to_string(),
        main_key: "Default".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "Slack", None, "dm", "User123", None);
    assert_eq!(key, "agent:myagent:default");
}

#[test]
fn test_whitespace_trimming() {
    let cfg = SessionConfig {
        agent_id: "  myagent  ".to_string(),
        dm_scope: "  main  ".to_string(),
        main_key: "  default  ".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "  slack  ", None, "dm", "  user123  ", None);
    assert_eq!(key, "agent:myagent:default");
}

#[test]
fn test_identity_links_direct_match() {
    let mut links = HashMap::new();
    links.insert("canonical".to_string(), vec!["user123".to_string()]);

    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "per-peer".to_string(),
        main_key: "default".to_string(),
        identity_links: links,
    };

    let key = build_session_key(&cfg, "slack", None, "dm", "user123", None);
    assert_eq!(key, "agent:myagent:dm:canonical");
}

#[test]
fn test_identity_links_scoped_match() {
    let mut links = HashMap::new();
    links.insert("canonical".to_string(), vec!["slack:user123".to_string()]);

    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "per-peer".to_string(),
        main_key: "default".to_string(),
        identity_links: links,
    };

    let key = build_session_key(&cfg, "slack", None, "dm", "user123", None);
    assert_eq!(key, "agent:myagent:dm:canonical");
}

#[test]
fn test_identity_links_with_non_empty_dm_scope() {
    let mut links = HashMap::new();
    links.insert(
        "canonical".to_string(),
        vec!["user123".to_string(), "slack:user123".to_string()],
    );

    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "per-channel-peer".to_string(),
        main_key: "default".to_string(),
        identity_links: links,
    };

    let key = build_session_key(&cfg, "slack", None, "dm", "user123", None);
    assert_eq!(key, "agent:myagent:slack:dm:canonical");
}

#[test]
fn test_no_identity_link_match() {
    let mut links = HashMap::new();
    links.insert("canonical".to_string(), vec!["other_user".to_string()]);

    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "per-peer".to_string(),
        main_key: "default".to_string(),
        identity_links: links,
    };

    let key = build_session_key(&cfg, "slack", None, "dm", "user123", None);
    assert_eq!(key, "agent:myagent:dm:user123");
}

#[test]
fn test_multiple_identity_link_values() {
    let mut links = HashMap::new();
    links.insert(
        "canonical".to_string(),
        vec![
            "other_user".to_string(),
            "user123".to_string(),
            "slack:other".to_string(),
        ],
    );

    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "per-peer".to_string(),
        main_key: "default".to_string(),
        identity_links: links,
    };

    let key = build_session_key(&cfg, "slack", None, "dm", "user123", None);
    assert_eq!(key, "agent:myagent:dm:canonical");
}

#[test]
fn test_resolve_identity_link_direct() {
    let mut links = HashMap::new();
    links.insert("canonical".to_string(), vec!["user123".to_string()]);

    let result = resolve_identity_link(&links, "slack", "user123");
    assert_eq!(result, Some("canonical".to_string()));
}

#[test]
fn test_resolve_identity_link_scoped() {
    let mut links = HashMap::new();
    links.insert("canonical".to_string(), vec!["slack:user123".to_string()]);

    let result = resolve_identity_link(&links, "slack", "user123");
    assert_eq!(result, Some("canonical".to_string()));
}

#[test]
fn test_resolve_identity_link_not_found() {
    let mut links = HashMap::new();
    links.insert("canonical".to_string(), vec!["other_user".to_string()]);

    let result = resolve_identity_link(&links, "slack", "user123");
    assert!(result.is_none());
}

#[test]
fn test_resolve_identity_link_empty_peer_id() {
    let mut links = HashMap::new();
    links.insert("canonical".to_string(), vec!["user123".to_string()]);

    let result = resolve_identity_link(&links, "slack", "");
    assert!(result.is_none());
}

#[test]
fn test_resolve_identity_link_empty_links() {
    let links: HashMap<String, Vec<String>> = HashMap::new();

    let result = resolve_identity_link(&links, "slack", "user123");
    assert!(result.is_none());
}

#[test]
fn test_default_account_id_when_none() {
    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "per-account-channel-peer".to_string(),
        main_key: "default".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "slack", None, "dm", "user123", None);
    assert_eq!(key, "agent:myagent:slack:default:dm:user123");
}

#[test]
fn test_whatsapp_channel() {
    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "main".to_string(),
        main_key: "default".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "whatsapp", None, "dm", "+1234567890", None);
    assert_eq!(key, "agent:myagent:default");
}

#[test]
fn test_special_characters_in_peer_id() {
    let cfg = SessionConfig {
        agent_id: "myagent".to_string(),
        dm_scope: "main".to_string(),
        main_key: "default".to_string(),
        identity_links: HashMap::new(),
    };
    let key = build_session_key(&cfg, "slack", None, "dm", "U123_abc-xyz", None);
    assert_eq!(key, "agent:myagent:default");
}
