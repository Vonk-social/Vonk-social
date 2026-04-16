//! Web Push dispatch — sends notifications to subscribed browsers/devices.
//!
//! Uses the `web-push` crate for RFC 8291 payload encryption + VAPID JWT.
//! Subscriptions are stored in `push_subscriptions`; this module handles
//! the "encrypt + POST to the browser's push endpoint" step.
//!
//! Three notification sources:
//!   - New DM message → notify recipient if notify_dm = true
//!   - Mention in a post → notify mentioned user if notify_mention = true
//!   - New follower → notify the followed user if notify_follow = true

use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::PgPool;

use crate::config::AppConfig;

#[derive(Debug, Serialize)]
pub struct PushPayload {
    pub title: String,
    pub body: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// Send a push notification to all subscriptions of a user that match
/// the notification type.
pub async fn notify_user(
    db: &PgPool,
    cfg: &AppConfig,
    user_id: i64,
    kind: NotifyKind,
    payload: &PushPayload,
) {
    if !cfg.vapid_configured() {
        return;
    }

    let filter_col = match kind {
        NotifyKind::Dm => "notify_dm",
        NotifyKind::Mention => "notify_mention",
        NotifyKind::Follow => "notify_follow",
        NotifyKind::Reply => "notify_reply",
    };

    // Fetch all subscriptions for this user that have the relevant flag on.
    let subs: Vec<PushSub> = match sqlx::query_as::<_, PushSub>(&format!(
        "SELECT id, endpoint, p256dh, auth FROM push_subscriptions \
         WHERE user_id = $1 AND {filter_col} = true"
    ))
    .bind(user_id)
    .fetch_all(db)
    .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, user_id, "failed to fetch push subscriptions");
            return;
        }
    };

    if subs.is_empty() {
        return;
    }

    let payload_json = match serde_json::to_vec(payload) {
        Ok(j) => j,
        Err(e) => {
            tracing::warn!(error = %e, "failed to serialize push payload");
            return;
        }
    };

    for sub in subs {
        let db = db.clone();
        let cfg_vapid_private = cfg.vapid_private_key.clone();
        let cfg_vapid_subject = cfg.vapid_subject.clone();
        let payload_bytes = payload_json.clone();
        let sub_id = sub.id;

        // Fire-and-forget per subscription — don't block on slow endpoints.
        tokio::spawn(async move {
            match send_single(&cfg_vapid_private, &cfg_vapid_subject, &sub, &payload_bytes).await {
                Ok(()) => {
                    tracing::debug!(sub_id, "push sent");
                }
                Err(e) => {
                    let msg = format!("{e:#}");
                    tracing::warn!(sub_id, error = %msg, "push delivery failed");
                    // If the endpoint is gone (410 Gone or 404), remove the subscription.
                    if msg.contains("410") || msg.contains("404") || msg.contains("Gone") {
                        let _ = sqlx::query("DELETE FROM push_subscriptions WHERE id = $1")
                            .bind(sub_id)
                            .execute(&db)
                            .await;
                        tracing::info!(sub_id, "removed expired push subscription");
                    }
                }
            }
        });
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NotifyKind {
    Dm,
    Mention,
    Follow,
    Reply,
}

#[derive(Debug, sqlx::FromRow)]
struct PushSub {
    id: i64,
    endpoint: String,
    p256dh: String,
    auth: String,
}

async fn send_single(
    vapid_private_key: &str,
    vapid_subject: &str,
    sub: &PushSub,
    payload: &[u8],
) -> Result<()> {
    use web_push::{
        ContentEncoding, IsahcWebPushClient, SubscriptionInfo, SubscriptionKeys,
        VapidSignatureBuilder, WebPushClient, WebPushMessageBuilder,
    };

    let subscription = SubscriptionInfo {
        endpoint: sub.endpoint.clone(),
        keys: SubscriptionKeys {
            p256dh: sub.p256dh.clone(),
            auth: sub.auth.clone(),
        },
    };

    let mut sig_builder = VapidSignatureBuilder::from_base64(
        vapid_private_key,
        web_push::URL_SAFE_NO_PAD,
        &subscription,
    )
    .context("build vapid signature")?;
    sig_builder.add_claim("sub", vapid_subject);
    let sig = sig_builder.build().context("finalize vapid signature")?;

    let mut builder = WebPushMessageBuilder::new(&subscription);
    builder.set_payload(ContentEncoding::Aes128Gcm, payload);
    builder.set_vapid_signature(sig);
    let message = builder.build().context("build push message")?;

    let client = IsahcWebPushClient::new().context("build web push client")?;
    client.send(message).await.context("dispatch web push")?;
    Ok(())
}
