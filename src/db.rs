use anyhow::Result;
use chrono::{DateTime, Utc, TimeZone};
use serde::{Deserialize, Serialize};
use sqlx::{AnyPool, Row};
use std::borrow::Cow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbKind {
    Sqlite,
    Postgres,
}

pub fn db_kind_from_url(url: &str) -> DbKind {
    let lower = url.to_lowercase();
    if lower.starts_with("postgres://") || lower.starts_with("postgresql://") {
        DbKind::Postgres
    } else {
        DbKind::Sqlite
    }
}

pub fn rewrite_sql<'a>(sql: &'a str, kind: DbKind) -> Cow<'a, str> {
    match kind {
        DbKind::Sqlite => Cow::Borrowed(sql),
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
            Cow::Owned(out)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_key: String,
    pub agent_id: String,
    pub business_profile_id: Option<String>,
    pub user_id: Option<String>,
    pub last_route: Option<serde_json::Value>,
    pub dm_scope: String,
    pub identity_links: Option<serde_json::Value>,
    #[serde(skip)]
    pub created_at: DateTime<Utc>,
    #[serde(skip)]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub session_key: String,
    pub direction: String,
    pub channel: String,
    pub account_id: Option<String>,
    pub peer_id: Option<String>,
    pub content: Option<String>,
    pub attachments: Option<serde_json::Value>,
    pub status: String,
    pub dedupe_key: Option<String>,
    #[serde(skip)]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxRecord {
    pub id: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub retry_count: i32,
    #[serde(skip)]
    pub next_attempt_at: DateTime<Utc>,
    pub last_error: Option<String>,
    #[serde(skip)]
    pub created_at: DateTime<Utc>,
}

fn i64_to_datetime(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(ts, 0).single().unwrap_or_else(|| Utc.timestamp_opt(ts, 0).earliest().unwrap_or(Utc::now()))
}

fn datetime_to_i64(dt: DateTime<Utc>) -> i64 {
    dt.timestamp()
}

pub async fn init_db(pool: &AnyPool, kind: DbKind) -> Result<()> {
    let stmts = vec![
        r#"CREATE TABLE IF NOT EXISTS sessions (
            session_key TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            business_profile_id TEXT,
            user_id TEXT,
            last_route TEXT,
            dm_scope TEXT NOT NULL,
            identity_links TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_key TEXT NOT NULL,
            direction TEXT NOT NULL,
            channel TEXT NOT NULL,
            account_id TEXT,
            peer_id TEXT,
            content TEXT,
            attachments TEXT,
            status TEXT NOT NULL,
            dedupe_key TEXT,
            created_at INTEGER NOT NULL
        )"#,
        r#"CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_key, created_at)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_messages_dedupe ON messages(dedupe_key)"#,
        r#"CREATE TABLE IF NOT EXISTS deliveries (
            id TEXT PRIMARY KEY,
            message_id TEXT NOT NULL,
            channel TEXT NOT NULL,
            status TEXT NOT NULL,
            attempts INTEGER NOT NULL,
            last_error TEXT,
            created_at INTEGER NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS inbound_outbox (
            id TEXT PRIMARY KEY,
            payload TEXT NOT NULL,
            status TEXT NOT NULL,
            retry_count INTEGER NOT NULL,
            next_attempt_at INTEGER NOT NULL,
            last_error TEXT,
            created_at INTEGER NOT NULL
        )"#,
        r#"CREATE INDEX IF NOT EXISTS idx_outbox_status ON inbound_outbox(status, next_attempt_at)"#,
        r#"CREATE TABLE IF NOT EXISTS pairing_requests (
            id TEXT PRIMARY KEY,
            channel TEXT NOT NULL,
            peer_id TEXT NOT NULL,
            code TEXT NOT NULL,
            expires_at INTEGER NOT NULL,
            status TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )"#,
    ];

    for stmt in stmts {
        let sql = rewrite_sql(stmt, kind);
        sqlx::query(sql.as_ref()).execute(pool).await?;
    }

    Ok(())
}

pub async fn upsert_session(pool: &AnyPool, kind: DbKind, record: &SessionRecord) -> Result<()> {
    let sql = rewrite_sql(
        r#"INSERT INTO sessions (
            session_key, agent_id, business_profile_id, user_id, last_route, dm_scope, identity_links, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(session_key) DO UPDATE SET
            agent_id=excluded.agent_id,
            business_profile_id=excluded.business_profile_id,
            user_id=excluded.user_id,
            last_route=excluded.last_route,
            dm_scope=excluded.dm_scope,
            identity_links=excluded.identity_links,
            updated_at=excluded.updated_at"#,
        kind,
    );
    sqlx::query(sql.as_ref())
        .bind(&record.session_key)
        .bind(&record.agent_id)
        .bind(record.business_profile_id.as_deref())
        .bind(record.user_id.as_deref())
        .bind(record.last_route.as_ref().map(|v| v.to_string()))
        .bind(&record.dm_scope)
        .bind(record.identity_links.as_ref().map(|v| v.to_string()))
        .bind(datetime_to_i64(record.created_at))
        .bind(datetime_to_i64(record.updated_at))
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_message(pool: &AnyPool, kind: DbKind, record: &MessageRecord) -> Result<()> {
    let sql = rewrite_sql(
        r#"INSERT INTO messages (
            id, session_key, direction, channel, account_id, peer_id, content, attachments, status, dedupe_key, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        kind,
    );
    sqlx::query(sql.as_ref())
        .bind(&record.id)
        .bind(&record.session_key)
        .bind(&record.direction)
        .bind(&record.channel)
        .bind(record.account_id.as_deref())
        .bind(record.peer_id.as_deref())
        .bind(record.content.as_deref())
        .bind(record.attachments.as_ref().map(|v| v.to_string()))
        .bind(&record.status)
        .bind(record.dedupe_key.as_deref())
        .bind(datetime_to_i64(record.created_at))
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn message_dedupe_exists(pool: &AnyPool, kind: DbKind, dedupe_key: &str) -> Result<bool> {
    let sql = rewrite_sql("SELECT 1 FROM messages WHERE dedupe_key = ? LIMIT 1", kind);
    let row = sqlx::query(sql.as_ref())
        .bind(dedupe_key)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

pub async fn list_sessions(pool: &AnyPool, kind: DbKind, limit: i64, offset: i64) -> Result<Vec<SessionRecord>> {
    let sql = rewrite_sql(
        r#"SELECT session_key, agent_id, business_profile_id, user_id, last_route, dm_scope, identity_links, created_at, updated_at
           FROM sessions ORDER BY updated_at DESC LIMIT ? OFFSET ?"#,
        kind,
    );
    let rows = sqlx::query(sql.as_ref())
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let mut result = Vec::new();
    for row in rows {
        let last_route: Option<String> = row.try_get("last_route")?;
        let identity_links: Option<String> = row.try_get("identity_links")?;
        let created_at: i64 = row.try_get("created_at")?;
        let updated_at: i64 = row.try_get("updated_at")?;
        result.push(SessionRecord {
            session_key: row.try_get("session_key")?,
            agent_id: row.try_get("agent_id")?,
            business_profile_id: row.try_get("business_profile_id")?,
            user_id: row.try_get("user_id")?,
            last_route: last_route.and_then(|v| serde_json::from_str(&v).ok()),
            dm_scope: row.try_get("dm_scope")?,
            identity_links: identity_links.and_then(|v| serde_json::from_str(&v).ok()),
            created_at: i64_to_datetime(created_at),
            updated_at: i64_to_datetime(updated_at),
        });
    }
    Ok(result)
}

pub async fn get_session(pool: &AnyPool, kind: DbKind, session_key: &str) -> Result<Option<SessionRecord>> {
    let sql = rewrite_sql(
        r#"SELECT session_key, agent_id, business_profile_id, user_id, last_route, dm_scope, identity_links, created_at, updated_at
           FROM sessions WHERE session_key = ?"#,
        kind,
    );
    let row = sqlx::query(sql.as_ref())
        .bind(session_key)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = row {
        let last_route: Option<String> = row.try_get("last_route")?;
        let identity_links: Option<String> = row.try_get("identity_links")?;
        let created_at: i64 = row.try_get("created_at")?;
        let updated_at: i64 = row.try_get("updated_at")?;
        return Ok(Some(SessionRecord {
            session_key: row.try_get("session_key")?,
            agent_id: row.try_get("agent_id")?,
            business_profile_id: row.try_get("business_profile_id")?,
            user_id: row.try_get("user_id")?,
            last_route: last_route.and_then(|v| serde_json::from_str(&v).ok()),
            dm_scope: row.try_get("dm_scope")?,
            identity_links: identity_links.and_then(|v| serde_json::from_str(&v).ok()),
            created_at: i64_to_datetime(created_at),
            updated_at: i64_to_datetime(updated_at),
        }));
    }
    Ok(None)
}

pub async fn list_messages(pool: &AnyPool, kind: DbKind, session_key: &str, limit: i64, offset: i64) -> Result<Vec<MessageRecord>> {
    let sql = rewrite_sql(
        r#"SELECT id, session_key, direction, channel, account_id, peer_id, content, attachments, status, dedupe_key, created_at
           FROM messages WHERE session_key = ? ORDER BY created_at DESC LIMIT ? OFFSET ?"#,
        kind,
    );
    let rows = sqlx::query(sql.as_ref())
        .bind(session_key)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let mut result = Vec::new();
    for row in rows {
        let attachments: Option<String> = row.try_get("attachments")?;
        let created_at: i64 = row.try_get("created_at")?;
        result.push(MessageRecord {
            id: row.try_get("id")?,
            session_key: row.try_get("session_key")?,
            direction: row.try_get("direction")?,
            channel: row.try_get("channel")?,
            account_id: row.try_get("account_id")?,
            peer_id: row.try_get("peer_id")?,
            content: row.try_get("content")?,
            attachments: attachments.and_then(|v| serde_json::from_str(&v).ok()),
            status: row.try_get("status")?,
            dedupe_key: row.try_get("dedupe_key")?,
            created_at: i64_to_datetime(created_at),
        });
    }
    Ok(result)
}

pub async fn insert_outbox(pool: &AnyPool, kind: DbKind, payload: serde_json::Value, next_attempt_at: DateTime<Utc>) -> Result<OutboxRecord> {
    let record = OutboxRecord {
        id: Uuid::new_v4().to_string(),
        payload: payload.clone(),
        status: "pending".to_string(),
        retry_count: 0,
        next_attempt_at,
        last_error: None,
        created_at: Utc::now(),
    };
    let sql = rewrite_sql(
        r#"INSERT INTO inbound_outbox (id, payload, status, retry_count, next_attempt_at, last_error, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        kind,
    );
    sqlx::query(sql.as_ref())
        .bind(&record.id)
        .bind(record.payload.to_string())
        .bind(&record.status)
        .bind(record.retry_count)
        .bind(datetime_to_i64(record.next_attempt_at))
        .bind(record.last_error.as_deref())
        .bind(datetime_to_i64(record.created_at))
        .execute(pool)
        .await?;
    Ok(record)
}

pub async fn claim_outbox_batch(pool: &AnyPool, kind: DbKind, now: DateTime<Utc>, limit: i64) -> Result<Vec<OutboxRecord>> {
    let now_i64 = datetime_to_i64(now);
    let sql = rewrite_sql(
        r#"SELECT id, payload, status, retry_count, next_attempt_at, last_error, created_at
           FROM inbound_outbox
           WHERE status IN ('pending','failed') AND next_attempt_at <= ?
           ORDER BY created_at ASC
           LIMIT ?"#,
        kind,
    );
    let rows = sqlx::query(sql.as_ref())
        .bind(now_i64)
        .bind(limit)
        .fetch_all(pool)
        .await?;

    let mut result = Vec::new();
    for row in rows {
        let payload: String = row.try_get("payload")?;
        let next_attempt_at: i64 = row.try_get("next_attempt_at")?;
        let created_at: i64 = row.try_get("created_at")?;
        result.push(OutboxRecord {
            id: row.try_get("id")?,
            payload: serde_json::from_str(&payload).unwrap_or_else(|_| serde_json::json!({})),
            status: row.try_get("status")?,
            retry_count: row.try_get::<i64, _>("retry_count")? as i32,
            next_attempt_at: i64_to_datetime(next_attempt_at),
            last_error: row.try_get("last_error")?,
            created_at: i64_to_datetime(created_at),
        });
    }

    if !result.is_empty() {
        let ids: Vec<String> = result.iter().map(|r| r.id.clone()).collect();
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let base_sql = format!("UPDATE inbound_outbox SET status='sending', last_error=NULL WHERE id IN ({})", placeholders);
        let update_sql = rewrite_sql(&base_sql, kind);
        let mut query = sqlx::query(update_sql.as_ref());
        for id in ids {
            query = query.bind(id);
        }
        query.execute(pool).await?;
    }

    Ok(result)
}

pub async fn mark_outbox_delivered(pool: &AnyPool, kind: DbKind, id: &str) -> Result<()> {
    let sql = rewrite_sql("UPDATE inbound_outbox SET status='delivered' WHERE id = ?", kind);
    sqlx::query(sql.as_ref()).bind(id).execute(pool).await?;
    Ok(())
}

pub async fn mark_outbox_failed(pool: &AnyPool, kind: DbKind, id: &str, retry_count: i32, next_attempt_at: DateTime<Utc>, error: &str) -> Result<()> {
    let sql = rewrite_sql(
        "UPDATE inbound_outbox SET status='failed', retry_count=?, next_attempt_at=?, last_error=? WHERE id=?",
        kind,
    );
    sqlx::query(sql.as_ref())
        .bind(retry_count)
        .bind(datetime_to_i64(next_attempt_at))
        .bind(error)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}