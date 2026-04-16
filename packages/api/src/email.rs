//! SMTP sender. Uses Postal (post.wattify.be) for outbound mail.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use lettre::message::{header::ContentType, Mailbox, Message};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParametersBuilder};
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

    // Accept invalid/expired certs — Postal's TLS cert on post.wattify.be
    // is expired since July 2025. TODO: remove once the cert is renewed.
    let tls = TlsParametersBuilder::new(cfg.smtp_host.clone())
        .dangerous_accept_invalid_certs(true)
        .build_rustls()
        .context("build TLS params")?;

    let transport: AsyncSmtpTransport<Tokio1Executor> = if cfg.smtp_port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.smtp_host)
            .context("build relay")?
            .port(cfg.smtp_port)
            .credentials(creds)
            .tls(Tls::Wrapper(tls))
            .timeout(Some(Duration::from_secs(15)))
            .build()
    } else {
        // Port 25 or 587: opportunistic STARTTLS with lenient cert check
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.smtp_host)
            .port(cfg.smtp_port)
            .credentials(creds)
            .tls(Tls::Opportunistic(tls))
            .timeout(Some(Duration::from_secs(15)))
            .build()
    };

    transport.send(email).await.context("send email")?;
    Ok(())
}
