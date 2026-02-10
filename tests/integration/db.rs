use agent_ping::db::{self, DbKind, SessionRecord, MessageRecord};
use chrono::Utc;
use serde_json::json;
use sqlx::AnyPool;
use std::fs;
use tempfile::TempDir;

async fn create_test_pool(db_path: &str) -> (AnyPool, DbKind) {
    sqlx::any::install_default_drivers();
    let db_url = format!("sqlite:///{}", db_path);
    let pool = AnyPool::connect(&db_url).await.unwrap();
    let kind = DbKind::Sqlite;
    db::init_db(&pool, kind).await.unwrap();
    (pool, kind)
}

#[tokio::test]
async fn test_db_init_and_tables() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, _kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sessions, 0);

    let messages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(messages, 0);
}

#[tokio::test]
async fn test_upsert_session_new() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let record = SessionRecord {
        session_key: "agent:slack:C123:U456".to_string(),
        agent_id: "main".to_string(),
        business_profile_id: Some("bp_123".to_string()),
        user_id: Some("user_1".to_string()),
        last_route: Some(json!({"channel": "slack", "account_id": "C123", "peer_id": "U456"})),
        dm_scope: "per-peer".to_string(),
        identity_links: Some(json!({"email": ["user@example.com"]})),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    db::upsert_session(&pool, kind, &record).await.unwrap();

    let retrieved = db::get_session(&pool, kind, &record.session_key).await.unwrap();
    assert!(retrieved.is_some());
    let session = retrieved.unwrap();
    assert_eq!(session.session_key, record.session_key);
    assert_eq!(session.agent_id, "main");
    assert_eq!(session.business_profile_id, Some("bp_123".to_string()));
}

#[tokio::test]
async fn test_upsert_session_update() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let record = SessionRecord {
        session_key: "agent:telegram:123456789".to_string(),
        agent_id: "main".to_string(),
        business_profile_id: None,
        user_id: None,
        last_route: None,
        dm_scope: "main".to_string(),
        identity_links: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    db::upsert_session(&pool, kind, &record).await.unwrap();

    let updated_record = SessionRecord {
        session_key: record.session_key.clone(),
        agent_id: "updated_agent".to_string(),
        business_profile_id: Some("bp_updated".to_string()),
        user_id: Some("user_updated".to_string()),
        last_route: Some(json!({"channel": "telegram"})),
        dm_scope: "main".to_string(),
        identity_links: Some(json!({"phone": ["+1234567890"]})),
        created_at: record.created_at,
        updated_at: Utc::now(),
    };

    db::upsert_session(&pool, kind, &updated_record).await.unwrap();

    let retrieved = db::get_session(&pool, kind, &record.session_key).await.unwrap();
    assert!(retrieved.is_some());
    let session = retrieved.unwrap();
    assert_eq!(session.agent_id, "updated_agent");
    assert_eq!(session.business_profile_id, Some("bp_updated".to_string()));
}

#[tokio::test]
async fn test_get_session_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let result = db::get_session(&pool, kind, "nonexistent_session").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_insert_message() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let record = MessageRecord {
        id: "msg_123".to_string(),
        session_key: "agent:slack:C123:U456".to_string(),
        direction: "inbound".to_string(),
        channel: "slack".to_string(),
        account_id: Some("C123".to_string()),
        peer_id: Some("U456".to_string()),
        content: Some("Hello, world!".to_string()),
        attachments: Some(json!([])),
        status: "received".to_string(),
        dedupe_key: Some("slack:U456:msg_123".to_string()),
        created_at: Utc::now(),
    };

    db::insert_message(&pool, kind, &record).await.unwrap();

    let messages = db::list_messages(&pool, kind, &record.session_key, 10, 0).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, "msg_123");
    assert_eq!(messages[0].content, Some("Hello, world!".to_string()));
}

#[tokio::test]
async fn test_list_messages_pagination() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let session_key = "agent:whatsapp:biz_123:+1234567890".to_string();

    for i in 0..5 {
        let record = MessageRecord {
            id: format!("msg_{}", i),
            session_key: session_key.clone(),
            direction: "outbound".to_string(),
            channel: "whatsapp".to_string(),
            account_id: Some("biz_123".to_string()),
            peer_id: Some("+1234567890".to_string()),
            content: Some(format!("Message {}", i)),
            attachments: None,
            status: "queued".to_string(),
            dedupe_key: None,
            created_at: Utc::now(),
        };
        db::insert_message(&pool, kind, &record).await.unwrap();
    }

    let all = db::list_messages(&pool, kind, &session_key, 10, 0).await.unwrap();
    assert_eq!(all.len(), 5);

    let first_two = db::list_messages(&pool, kind, &session_key, 2, 0).await.unwrap();
    assert_eq!(first_two.len(), 2);

    let skip_two = db::list_messages(&pool, kind, &session_key, 10, 2).await.unwrap();
    assert_eq!(skip_two.len(), 3);
}

#[tokio::test]
async fn test_message_dedupe_exists() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let dedupe_key = "telegram:123456789:msg_abc";
    let exists = db::message_dedupe_exists(&pool, kind, dedupe_key).await.unwrap();
    assert!(!exists);

    let record = MessageRecord {
        id: "msg_abc".to_string(),
        session_key: "agent:telegram:123456789".to_string(),
        direction: "inbound".to_string(),
        channel: "telegram".to_string(),
        account_id: None,
        peer_id: Some("123456789".to_string()),
        content: Some("Test".to_string()),
        attachments: None,
        status: "received".to_string(),
        dedupe_key: Some(dedupe_key.to_string()),
        created_at: Utc::now(),
    };
    db::insert_message(&pool, kind, &record).await.unwrap();

    let exists = db::message_dedupe_exists(&pool, kind, dedupe_key).await.unwrap();
    assert!(exists);
}

#[tokio::test]
async fn test_list_sessions() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let sessions = vec![
        "agent:slack:C1:U1",
        "agent:slack:C1:U2",
        "agent:telegram:tg_123",
    ];

    for key in &sessions {
        let record = SessionRecord {
            session_key: key.to_string(),
            agent_id: "main".to_string(),
            business_profile_id: None,
            user_id: None,
            last_route: None,
            dm_scope: "main".to_string(),
            identity_links: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        db::upsert_session(&pool, kind, &record).await.unwrap();
    }

    let all = db::list_sessions(&pool, kind, 100, 0).await.unwrap();
    assert_eq!(all.len(), 3);

    let first_two = db::list_sessions(&pool, kind, 2, 0).await.unwrap();
    assert_eq!(first_two.len(), 2);
}

#[tokio::test]
async fn test_insert_outbox() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let payload = json!({
        "session_key": "agent:slack:C123:U456",
        "channel": "slack",
        "text": "Test message"
    });
    let next_attempt = Utc::now();

    let record = db::insert_outbox(&pool, kind, payload.clone(), next_attempt).await.unwrap();
    assert!(!record.id.is_empty());
    assert_eq!(record.status, "pending");
    assert_eq!(record.retry_count, 0);
}

#[tokio::test]
async fn test_claim_outbox_batch() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let now = Utc::now();
    let past = now - chrono::Duration::hours(1);

    for i in 0..3 {
        let payload = json!({"index": i});
        let _ = db::insert_outbox(&pool, kind, payload, past).await.unwrap();
    }

    let claimed = db::claim_outbox_batch(&pool, kind, now, 2).await.unwrap();
    assert_eq!(claimed.len(), 2);

    for record in &claimed {
        assert_eq!(record.status, "sending");
    }

    let remaining = db::claim_outbox_batch(&pool, kind, now, 10).await.unwrap();
    assert_eq!(remaining.len(), 1);
}

#[tokio::test]
async fn test_mark_outbox_delivered() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let payload = json!({"test": true});
    let record = db::insert_outbox(&pool, kind, payload, Utc::now()).await.unwrap();

    db::mark_outbox_delivered(&pool, kind, &record.id).await.unwrap();

    let claimed = db::claim_outbox_batch(&pool, kind, Utc::now(), 10).await.unwrap();
    assert!(claimed.is_empty());
}

#[tokio::test]
async fn test_mark_outbox_failed() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let payload = json!({"test": true});
    let record = db::insert_outbox(&pool, kind, payload, Utc::now()).await.unwrap();

    let next_attempt = Utc::now() + chrono::Duration::hours(1);
    db::mark_outbox_failed(&pool, kind, &record.id, 1, next_attempt, "Connection refused")
        .await
        .unwrap();

    let claimed = db::claim_outbox_batch(&pool, kind, Utc::now(), 10).await.unwrap();
    assert!(claimed.is_empty());

    let past = Utc::now() + chrono::Duration::hours(2);
    let retry = db::claim_outbox_batch(&pool, kind, past, 10).await.unwrap();
    assert_eq!(retry.len(), 1);
    assert_eq!(retry[0].status, "failed");
    assert_eq!(retry[0].retry_count, 1);
}

#[tokio::test]
async fn test_outbox_claim_with_mixed_status() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let now = Utc::now();
    let past = now - chrono::Duration::hours(1);

    let pending = db::insert_outbox(&pool, kind, json!({"status": "pending"}), past)
        .await
        .unwrap();
    let _ = db::insert_outbox(&pool, kind, json!({"status": "delivered"}), past)
        .await
        .unwrap();
    let failed = db::insert_outbox(&pool, kind, json!({"status": "failed"}), past)
        .await
        .unwrap();

    let claimed = db::claim_outbox_batch(&pool, kind, now, 10).await.unwrap();
    assert_eq!(claimed.len(), 2);

    let ids: Vec<String> = claimed.iter().map(|r| r.id.clone()).collect();
    assert!(ids.contains(&pending.id));
    assert!(ids.contains(&failed.id));
}

#[tokio::test]
async fn test_session_with_identity_links_null() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let record = SessionRecord {
        session_key: "agent:no_identities".to_string(),
        agent_id: "main".to_string(),
        business_profile_id: None,
        user_id: None,
        last_route: None,
        dm_scope: "main".to_string(),
        identity_links: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    db::upsert_session(&pool, kind, &record).await.unwrap();

    let retrieved = db::get_session(&pool, kind, &record.session_key).await.unwrap();
    assert!(retrieved.is_some());
    assert!(retrieved.unwrap().identity_links.is_none());
}

#[tokio::test]
async fn test_message_with_attachments_json() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let (pool, kind) = create_test_pool(db_path.to_str().unwrap()).await;

    let attachments = json!([
        {"id": "att1", "url": "https://example.com/image.jpg", "mime_type": "image/jpeg"},
        {"id": "att2", "url": "https://example.com/doc.pdf", "mime_type": "application/pdf"}
    ]);

    let record = MessageRecord {
        id: "msg_with_attachments".to_string(),
        session_key: "agent:slack:C123:U456".to_string(),
        direction: "inbound".to_string(),
        channel: "slack".to_string(),
        account_id: Some("C123".to_string()),
        peer_id: Some("U456".to_string()),
        content: Some("Message with attachments".to_string()),
        attachments: Some(attachments),
        status: "received".to_string(),
        dedupe_key: None,
        created_at: Utc::now(),
    };

    db::insert_message(&pool, kind, &record).await.unwrap();

    let messages = db::list_messages(&pool, kind, &record.session_key, 1, 0).await.unwrap();
    assert_eq!(messages.len(), 1);
    let retrieved = &messages[0];
    assert!(retrieved.attachments.is_some());
    let att = retrieved.attachments.as_ref().unwrap();
    assert!(att.is_array());
    assert_eq!(att.as_array().unwrap().len(), 2);
}