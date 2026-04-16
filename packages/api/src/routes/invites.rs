//! Email invites + handle-based friend discovery (Phase 3).
//!
//! * `POST /api/invites`               — email someone an invite
//! * `GET  /api/invites/sent`          — list my outgoing invites
//! * `POST /api/invites/match-handles` — find existing Vonk users who
//!   registered with one of the IG / X / Snap / Telegram / Bluesky /
//!   Mastodon handles I supplied.
//!
//! Privacy posture:
//!   - Handles are matched case-insensitively against what each user
//!     chose to put on their own profile. We never scrape external
//!     platforms, never store the submitter's address book, and reject
//!     handles longer than 60 chars.
//!   - Invite emails are sent via Postal. Sender is `noreply@vonk.social`
//!     — we don't expose the inviter's email address to the recipient.
//!   - One outstanding invite per (sender, recipient) is enforced at
//!     the DB level via a partial unique index.

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::auth::AuthUser;
use crate::email::{self, Outgoing};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/invites", post(create))
        .route("/api/invites/sent", get(list_sent))
        .route("/api/invites/match-handles", post(match_handles))
}

// ── POST /api/invites ────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct CreateInviteRequest {
    #[validate(email, length(max = 254))]
    pub email: String,
    #[validate(length(max = 500))]
    pub note: Option<String>,
}


async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<CreateInviteRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    req.validate()?;

    // If this email is already a Vonk user, don't send anything — let
    // the caller know so the frontend can switch to a "follow" CTA.
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM users WHERE email = $1 AND deleted_at IS NULL",
    )
    .bind(&req.email)
    .fetch_optional(&state.db)
    .await?;
    if exists.is_some() {
        return Err(ApiError::conflict(
            "already_on_vonk",
            "This address is already registered on Vonk",
        ));
    }

    // Upsert pending invite.
    let row: (Uuid, Option<chrono::DateTime<chrono::Utc>>) = sqlx::query_as(
        "INSERT INTO email_invites (sender_id, recipient_email, note) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (sender_id, recipient_email) \
         WHERE accepted_at IS NULL AND failed_at IS NULL \
         DO UPDATE SET note = EXCLUDED.note \
         RETURNING uuid, sent_at",
    )
    .bind(user.id)
    .bind(&req.email)
    .bind(&req.note)
    .fetch_one(&state.db)
    .await?;
    let invite_uuid = row.0;
    let already_sent = row.1.is_some();

    if already_sent {
        return Ok(Json(serde_json::json!({
            "data": {
                "uuid": invite_uuid,
                "email": req.email,
                "sent": false,
                "message": "invite_already_sent",
            }
        })));
    }

    // Dispatch the mail in a background task so the HTTP response
    // returns immediately. SMTP can be slow (DNS, TLS handshake,
    // Postal queue) and we don't want a 504 gateway timeout.
    if state.config.smtp_configured() {
        let db = state.db.clone();
        let cfg = state.config.clone();
        let to_email = req.email.clone();
        let display = if user.display_name.is_empty() {
            user.username.clone()
        } else {
            user.display_name.clone()
        };
        let note = req.note.clone();
        tokio::spawn(async move {
            let text = invite_text(&display, note.as_deref(), invite_uuid);
            let html = invite_html(&display, note.as_deref(), invite_uuid);
            match email::send(
                &cfg,
                Outgoing {
                    to: to_email.clone(),
                    subject: format!("{display} nodigt je uit op Vonk"),
                    text_body: text,
                    html_body: Some(html),
                },
            )
            .await
            {
                Ok(()) => {
                    let _ = sqlx::query(
                        "UPDATE email_invites SET sent_at = now() WHERE uuid = $1",
                    )
                    .bind(invite_uuid)
                    .execute(&db)
                    .await;
                    tracing::info!(email = %to_email, "invite sent");
                }
                Err(e) => {
                    tracing::warn!(error = %e, chain = ?e.source(), email = %to_email, "invite send failed");
                    let _ = sqlx::query(
                        "UPDATE email_invites SET failed_at = now(), failure_reason = $1 WHERE uuid = $2",
                    )
                    .bind(format!("{e:#}"))
                    .bind(invite_uuid)
                    .execute(&db)
                    .await;
                }
            }
        });
    }

    // We always respond "queued" immediately — the background task
    // flips sent_at/failed_at async. The frontend can poll /sent if
    // it wants to show delivery status later.
    Ok(Json(serde_json::json!({
        "data": {
            "uuid": invite_uuid,
            "email": req.email,
            "sent": false,
            "message": "queued",
        }
    })))
}

// ── GET /api/invites/sent ────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SentInvite {
    pub uuid: Uuid,
    pub recipient_email: String,
    pub note: Option<String>,
    pub sent_at: Option<chrono::DateTime<chrono::Utc>>,
    pub accepted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub failed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub failure_reason: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn list_sent(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows: Vec<SentInvite> = sqlx::query_as(
        "SELECT uuid, recipient_email, note, sent_at, accepted_at, failed_at, failure_reason, created_at \
         FROM email_invites WHERE sender_id = $1 \
         ORDER BY created_at DESC LIMIT 200",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(serde_json::json!({ "data": rows })))
}

// ── POST /api/invites/match-handles ──────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct MatchHandlesRequest {
    #[serde(default)]
    pub instagram: Vec<String>,
    #[serde(default)]
    pub twitter: Vec<String>,
    #[serde(default)]
    pub snapchat: Vec<String>,
    #[serde(default)]
    pub telegram: Vec<String>,
    #[serde(default)]
    pub bluesky: Vec<String>,
    #[serde(default)]
    pub mastodon: Vec<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MatchedUser {
    pub uuid: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    /// Which platform handle triggered the match (first hit wins).
    pub matched_on: String,
}

async fn match_handles(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<MatchHandlesRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    fn clean(xs: Vec<String>) -> Vec<String> {
        xs.into_iter()
            .map(|s| s.trim().trim_start_matches('@').to_ascii_lowercase())
            .filter(|s| !s.is_empty() && s.len() <= 60)
            .take(200) // hard cap to keep query arrays small
            .collect()
    }

    let ig = clean(req.instagram);
    let tw = clean(req.twitter);
    let sn = clean(req.snapchat);
    let tg = clean(req.telegram);
    let bs = clean(req.bluesky);
    let ma = clean(req.mastodon);

    if ig.is_empty() && tw.is_empty() && sn.is_empty() && tg.is_empty() && bs.is_empty() && ma.is_empty()
    {
        let empty: Vec<MatchedUser> = Vec::new();
        return Ok(Json(serde_json::json!({ "data": empty })));
    }

    // Single query that case-insensitively matches any of the per-platform
    // arrays against the corresponding column, and tags which column hit
    // first. Excludes the requesting user themselves.
    let rows: Vec<MatchedUser> = sqlx::query_as(
        r#"
        SELECT DISTINCT ON (u.id)
               u.uuid, u.username, u.display_name, u.avatar_url,
               CASE
                 WHEN lower(u.handle_instagram) = ANY($1) THEN 'instagram'
                 WHEN lower(u.handle_twitter)   = ANY($2) THEN 'twitter'
                 WHEN lower(u.handle_snapchat)  = ANY($3) THEN 'snapchat'
                 WHEN lower(u.handle_telegram)  = ANY($4) THEN 'telegram'
                 WHEN lower(u.handle_bluesky)   = ANY($5) THEN 'bluesky'
                 WHEN lower(u.handle_mastodon)  = ANY($6) THEN 'mastodon'
               END AS matched_on
          FROM users u
         WHERE u.deleted_at IS NULL
           AND u.id <> $7
           AND (
               lower(u.handle_instagram) = ANY($1)
            OR lower(u.handle_twitter)   = ANY($2)
            OR lower(u.handle_snapchat)  = ANY($3)
            OR lower(u.handle_telegram)  = ANY($4)
            OR lower(u.handle_bluesky)   = ANY($5)
            OR lower(u.handle_mastodon)  = ANY($6)
           )
         LIMIT 200
        "#,
    )
    .bind(&ig)
    .bind(&tw)
    .bind(&sn)
    .bind(&tg)
    .bind(&bs)
    .bind(&ma)
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "data": rows })))
}

// ── templates ────────────────────────────────────────────────

fn invite_link(invite_uuid: Uuid) -> String {
    format!("https://vonk.social/invite/{invite_uuid}")
}

fn invite_text(inviter: &str, note: Option<&str>, invite_uuid: Uuid) -> String {
    let link = invite_link(invite_uuid);
    let note_line = note
        .map(|n| format!("\n{inviter} schrijft:\n\"{n}\"\n"))
        .unwrap_or_default();
    format!(
        "Hoi,\n\n{inviter} nodigt je uit op Vonk — een open-source sociaal platform zonder reclame, zonder tracking en zonder algoritme.\n{note_line}\nMaak een account via:\n{link}\n\nBedankt,\nHet Vonk-team\n\n—\nWil je deze uitnodigingen niet meer ontvangen? Negeer deze mail; we volgen geen opens of clicks.",
    )
}

fn invite_html(inviter: &str, note: Option<&str>, invite_uuid: Uuid) -> String {
    let link = invite_link(invite_uuid);
    let note_block = note
        .map(|n| {
            format!(
                "<blockquote style=\"border-left:3px solid #C2593A;margin:16px 0;padding:4px 12px;color:#2D1F14;\">{}</blockquote>",
                html_escape(n)
            )
        })
        .unwrap_or_default();
    format!(
        "<!doctype html><html><body style=\"font-family:system-ui,sans-serif;background:#FFF8F0;color:#2D1F14;padding:24px\">\
         <div style=\"max-width:520px;margin:auto;background:#fff;border-radius:20px;padding:28px\">\
         <h1 style=\"color:#C2593A;margin:0 0 16px\">Je bent uitgenodigd op Vonk</h1>\
         <p>{} nodigt je uit op Vonk — een open-source sociaal platform zonder reclame, zonder tracking en zonder algoritme.</p>\
         {}\
         <p style=\"margin:24px 0\"><a href=\"{}\" style=\"display:inline-block;background:#C2593A;color:#fff;padding:12px 24px;border-radius:999px;text-decoration:none\">Account aanmaken</a></p>\
         <p style=\"color:#8a7a6b;font-size:13px\">Wil je geen uitnodigingen meer ontvangen? Negeer deze mail — we volgen geen opens of clicks.</p>\
         </div></body></html>",
        html_escape(inviter),
        note_block,
        link,
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
