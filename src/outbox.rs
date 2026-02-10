use crate::config::BackendConfig;
use crate::db::{
    claim_outbox_batch, mark_outbox_delivered, mark_outbox_failed, DbKind, OutboxRecord,
};
use chrono::{Duration, Utc};
use reqwest::Client;
use sqlx::AnyPool;
use tokio::time::sleep;

const OUTBOX_POLL_SECONDS: u64 = 2;
const OUTBOX_BATCH: i64 = 25;
const OUTBOX_MAX_RETRIES: i32 = 10;

pub fn compute_backoff(retry_count: i32) -> Duration {
    let exponent = (retry_count.max(1) - 1).min(8) as u32;
    let base = 2_i64.pow(exponent);
    Duration::seconds((base * 5).min(300))
}

pub async fn start_outbox_worker(pool: AnyPool, backend: BackendConfig, db_kind: DbKind) {
    if backend.webhook_url.is_none() {
        return;
    }

    let client = Client::new();
    loop {
        let now = Utc::now();
        if let Ok(batch) = claim_outbox_batch(&pool, db_kind, now, OUTBOX_BATCH).await {
            for row in batch {
                if let Err(err) = dispatch_row(&client, &backend, &pool, db_kind, &row).await {
                    let retry = row.retry_count + 1;
                    if retry >= OUTBOX_MAX_RETRIES {
                        let _ = mark_outbox_failed(
                            &pool,
                            db_kind,
                            &row.id,
                            retry,
                            now + Duration::seconds(3600),
                            &err.to_string(),
                        )
                        .await;
                        continue;
                    }
                    let next = Utc::now() + compute_backoff(retry);
                    let _ = mark_outbox_failed(
                        &pool,
                        db_kind,
                        &row.id,
                        retry,
                        next,
                        &err.to_string(),
                    )
                    .await;
                }
            }
        }
        sleep(std::time::Duration::from_secs(OUTBOX_POLL_SECONDS)).await;
    }
}

async fn dispatch_row(
    client: &Client,
    backend: &BackendConfig,
    pool: &AnyPool,
    db_kind: DbKind,
    row: &OutboxRecord,
) -> anyhow::Result<()> {
    let url = backend.webhook_url.as_ref().expect("webhook_url exists");
    let mut req = client.post(url).json(&row.payload);
    if let Some(token) = backend.api_token.as_ref() {
        req = req.header("X-Agent-Ping-Token", token);
    }

    let resp = req.send().await?;
    if resp.status().is_success() {
        mark_outbox_delivered(pool, db_kind, &row.id).await?;
        return Ok(());
    }

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    Err(anyhow::anyhow!("backend webhook failed: {} {}", status, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_compute_backoff_zero() {
        let backoff = compute_backoff(0);
        assert_eq!(backoff, Duration::seconds(5));
    }

    #[test]
    fn test_compute_backoff_one() {
        let backoff = compute_backoff(1);
        assert_eq!(backoff, Duration::seconds(5));
    }

    #[test]
    fn test_compute_backoff_two() {
        let backoff = compute_backoff(2);
        assert_eq!(backoff, Duration::seconds(10));
    }

    #[test]
    fn test_compute_backoff_four() {
        let backoff = compute_backoff(4);
        assert_eq!(backoff, Duration::seconds(40));
    }

    #[test]
    fn test_compute_backoff_eight() {
        let backoff = compute_backoff(8);
        assert_eq!(backoff, Duration::seconds(300));
    }

    #[test]
    fn test_compute_backoff_nine() {
        let backoff = compute_backoff(9);
        assert_eq!(backoff, Duration::seconds(300));
    }

    #[test]
    fn test_compute_backoff_max() {
        let backoff = compute_backoff(100);
        assert_eq!(backoff, Duration::seconds(300));
    }

    #[test]
    fn test_compute_backoff_negative() {
        let backoff = compute_backoff(-1);
        assert_eq!(backoff, Duration::seconds(5));
    }
}
