use crate::types::{Attachment, InboundMessage, OutboundMessage, RouteInfo};

use anyhow::Context;
use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct RuntimeHeader {
    name: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct RuntimeIngestRequest {
    method: String,
    path: String,
    query: Option<String>,
    headers: Vec<RuntimeHeader>,
    body: String,
}

#[derive(Debug, Deserialize)]
pub struct RuntimeIngestResponse {
    pub body: String,
    pub content_type: Option<String>,
    pub messages: Vec<InboundMessage>,
    pub status: u16,
}

#[derive(Debug, Serialize)]
struct RuntimeSendRequest {
    account_id: Option<String>,
    attachments: Vec<Attachment>,
    peer_id: Option<String>,
    reply_to: Option<String>,
    text: Option<String>,
    thread_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RuntimeSendResponse {
    pub message_id: Option<String>,
}

pub async fn ingest(
    client: &reqwest::Client,
    runtime_url: &str,
    channel: &str,
    method: &str,
    path: &str,
    query: Option<&str>,
    headers: &HeaderMap,
    body: &[u8],
) -> anyhow::Result<RuntimeIngestResponse> {
    let request = RuntimeIngestRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: query.map(ToString::to_string),
        headers: headers_to_vec(headers),
        body: String::from_utf8_lossy(body).to_string(),
    };

    let response = client
        .post(format!(
            "{}/internal/adapters/{channel}/ingest",
            runtime_url.trim_end_matches('/')
        ))
        .json(&request)
        .send()
        .await
        .with_context(|| format!("failed to call embedded adapter ingest for {channel}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("embedded adapter ingest failed for {channel}: {status} {body}");
    }

    response
        .json::<RuntimeIngestResponse>()
        .await
        .with_context(|| format!("invalid embedded adapter ingest response for {channel}"))
}

pub async fn send(
    client: &reqwest::Client,
    runtime_url: &str,
    channel: &str,
    route: &RouteInfo,
    outbound: &OutboundMessage,
) -> anyhow::Result<RuntimeSendResponse> {
    let request = RuntimeSendRequest {
        account_id: route
            .account_id
            .clone()
            .or_else(|| outbound.account_id.clone()),
        attachments: outbound.attachments.clone(),
        peer_id: route.peer_id.clone().or_else(|| outbound.peer_id.clone()),
        reply_to: outbound.reply_to.clone(),
        text: outbound.text.clone(),
        thread_id: route.thread_id.clone(),
    };

    let response = client
        .post(format!(
            "{}/internal/adapters/{channel}/send",
            runtime_url.trim_end_matches('/')
        ))
        .json(&request)
        .send()
        .await
        .with_context(|| format!("failed to call embedded adapter send for {channel}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("embedded adapter send failed for {channel}: {status} {body}");
    }

    response
        .json::<RuntimeSendResponse>()
        .await
        .with_context(|| format!("invalid embedded adapter send response for {channel}"))
}

fn headers_to_vec(headers: &HeaderMap) -> Vec<RuntimeHeader> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value.to_str().ok().map(|value| RuntimeHeader {
                name: name.as_str().to_string(),
                value: value.to_string(),
            })
        })
        .collect()
}
