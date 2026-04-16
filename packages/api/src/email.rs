//! SMTP sender. Uses Postal (post.wattify.be) for outbound mail.
//!
//! Kept deliberately small: construct a transport per send. Connection
//! pooling via `AsyncSmtpTransport::pool_config` is a later optimization;
//! invites are low-volume and this keeps the error paths explicit.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use lettre::message::{header::ContentType, Mailbox, Message};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

use crate::config::AppConfig;

pub struct Outgoing {
    pub to: String,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
}

pub async fn send(cfg: &AppConfig, out: Outgoing) -> Result<()> {
    if !cfg.smtp_configured() {
        return Err(anyhow!("smtp_not_configured"));
    }

    let from: Mailbox = format!("{} <{}>", cfg.smtp_from_name, cfg.smtp_from)
        .parse()
        .context("parse SMTP From mailbox")?;
    let to: Mailbox = out.to.parse().context("parse recipient mailbox")?;

    let builder = Message::builder()
        .from(from)
        .to(to)
        .subject(&out.subject);

    let email = match out.html_body {
        Some(html) => builder
            .multipart(
                lettre::message::MultiPart::alternative_plain_html(out.text_body, html),
            )
            .context("build multipart message")?,
        None => builder
            .header(ContentType::TEXT_PLAIN)
            .body(out.text_body)
            .context("build plain message")?,
    };

    let creds = Credentials::new(cfg.smtp_user.clone(), cfg.smtp_pass.clone());

    // Postal accepts STARTTLS on 587 and plain/STARTTLS-opportunistic on 25.
    // Some hosting providers block outbound 587, so we adapt based on port.
    let transport: AsyncSmtpTransport<Tokio1Executor> = if cfg.smtp_port == 465 {
        // Implicit TLS (SMTPS)
        AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.smtp_host)
            .context("build TLS relay")?
            .port(cfg.smtp_port)
            .credentials(creds)
            .timeout(Some(Duration::from_secs(15)))
            .build()
    } else if cfg.smtp_port == 25 {
        // Port 25: try STARTTLS opportunistically, fall back to plain.
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.smtp_host)
            .context("build STARTTLS relay")?
            .port(cfg.smtp_port)
            .credentials(creds)
            .timeout(Some(Duration::from_secs(15)))
            .build()
    } else {
        // Port 587 (default): require STARTTLS
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.smtp_host)
            .context("build STARTTLS relay")?
            .port(cfg.smtp_port)
            .credentials(creds)
            .timeout(Some(Duration::from_secs(15)))
            .build()
    };

    transport.send(email).await.context("send email")?;
    Ok(())
}
