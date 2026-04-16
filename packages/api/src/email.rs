//! Email sender via Postal HTTP API.
//!
//! Uses Postal's `/api/v1/send/message` endpoint instead of SMTP.
//! Advantages over SMTP:
//! - No TLS cert issues (Postal's cert is expired)
//! - Return-Path automatically set by Postal to psrp1.vonk.social
//! - Faster (single HTTP POST vs SMTP handshake)
//! - DKIM signing handled server-side by Postal

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

pub struct Outgoing {
    pub to: String,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
}

#[derive(Serialize)]
struct PostalMessage {
    to: Vec<String>,
    from: String,
    sender: String,
    subject: String,
    plain_body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_body: Option<String>,
}

#[derive(Deserialize)]
struct PostalResponse {
    status: String,
    #[allow(dead_code)]
    data: serde_json::Value,
}

pub async fn send(cfg: &AppConfig, out: Outgoing) -> Result<()> {
    if !cfg.smtp_configured() {
        return Err(anyhow!("email_not_configured"));
    }

    let api_url = format!("https://{}/api/v1/send/message", cfg.smtp_host);

    let msg = PostalMessage {
        to: vec![out.to],
        from: format!("{} <{}>", cfg.smtp_from_name, cfg.smtp_from),
        sender: cfg.smtp_from.clone(),
        subject: out.subject,
        plain_body: out.text_body,
        html_body: out.html_body,
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("build http client")?;

    let res = client
        .post(&api_url)
        .header("X-Server-API-Key", &cfg.smtp_pass)
        .json(&msg)
        .send()
        .await
        .context("POST to Postal API")?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(anyhow!("Postal API returned {status}: {body}"));
    }

    let parsed: PostalResponse = res.json().await.context("parse Postal response")?;
    if parsed.status != "success" {
        return Err(anyhow!("Postal API status: {}", parsed.status));
    }

    Ok(())
}
