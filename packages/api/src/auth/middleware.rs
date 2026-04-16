//! `AuthUser` extractor.
//!
//! Protects a route by requiring a valid access JWT in either the
//! `vonk_access` cookie (browser) or an `Authorization: Bearer <jwt>` header
//! (future mobile client / `curl`).

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::cookie::CookieJar;

use crate::error::ApiError;
use crate::models::User;
use crate::state::AppState;

use super::cookies::ACCESS_COOKIE;
use super::jwt;

/// An authenticated user, loaded from the database on every request.
///
/// If the access token is missing, expired, malformed, or points at a
/// suspended or deleted user, the extractor returns
/// [`ApiError::Unauthenticated`] (HTTP 401) so the frontend can attempt
/// `POST /api/auth/refresh`.
#[derive(Debug, Clone)]
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_token(parts).ok_or(ApiError::Unauthenticated)?;

        let claims = jwt::verify(&token, &state.config).map_err(|_| ApiError::Unauthenticated)?;
        let user_id = claims.user_id().ok_or(ApiError::Unauthenticated)?;

        let user = sqlx::query_as::<_, User>(&format!(
            "SELECT {cols} FROM users WHERE id = $1 AND deleted_at IS NULL",
            cols = User::COLUMNS,
        ))
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::Unauthenticated)?;

        if user.is_suspended.unwrap_or(false) {
            return Err(ApiError::Forbidden);
        }

        Ok(AuthUser(user))
    }
}

/// Extract the access JWT from either the cookie jar or `Authorization: Bearer`.
fn extract_token(parts: &Parts) -> Option<String> {
    // 1. Authorization: Bearer ...
    if let Some(h) = parts.headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(s) = h.to_str() {
            if let Some(tok) = s.strip_prefix("Bearer ") {
                if !tok.is_empty() {
                    return Some(tok.to_string());
                }
            }
        }
    }

    // 2. vonk_access cookie.
    let jar = CookieJar::from_headers(&parts.headers);
    jar.get(ACCESS_COOKIE).map(|c| c.value().to_string())
}
