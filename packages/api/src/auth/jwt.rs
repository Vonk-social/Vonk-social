//! JWT minting and verification.
//!
//! Short-lived (15 min) HS256 access tokens. Refresh tokens are *not* JWTs —
//! they are opaque session UUIDs stored in the `sessions` table.

use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::models::User;

/// Access-token claims.
///
/// * `sub` — internal user id (BIGSERIAL) as a decimal string to preserve
///   interoperability with JWT convention of string subs.
/// * `sid` — session UUID in the `sessions` table (for revocation).
/// * `username` — denormalised so the frontend can render the top-bar without
///   an extra `/me` fetch.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub sid: String,
    pub username: String,
    pub iat: i64,
    pub exp: i64,
    pub iss: String,
}

impl AccessClaims {
    /// Parse `sub` back into the i64 user id.
    pub fn user_id(&self) -> Option<i64> {
        self.sub.parse().ok()
    }

    /// Parse `sid` back into a UUID. Used by routes that need to invalidate
    /// the current session on top of clearing cookies.
    #[allow(dead_code)] // part of the public API; first consumer lands in Phase 2
    pub fn session_id(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.sid).ok()
    }
}

/// Mint a new access token for the given user + session.
pub fn mint(user: &User, session_id: Uuid, cfg: &AppConfig) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let exp = now + chrono::Duration::from_std(cfg.jwt_access_ttl).unwrap_or(chrono::Duration::minutes(15));

    let claims = AccessClaims {
        sub: user.id.to_string(),
        sid: session_id.to_string(),
        username: user.username.clone(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
        iss: "vonk".into(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(cfg.jwt_secret.as_bytes()),
    )
}

/// Verify a token and return its claims.
///
/// Returns `Err` for any signature, expiry, issuer or format violation.
pub fn verify(token: &str, cfg: &AppConfig) -> Result<AccessClaims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.set_issuer(&["vonk"]);
    validation.validate_exp = true;
    let data = decode::<AccessClaims>(
        token,
        &DecodingKey::from_secret(cfg.jwt_secret.as_bytes()),
        &validation,
    )?;
    Ok(data.claims)
}
