//! Cookie builders for the session/refresh pair.
//!
//! * `vonk_access` — httpOnly JWT, Path=/, short-lived (`jwt_access_ttl`).
//! * `vonk_refresh` — httpOnly opaque session UUID, Path=/api/auth, long-lived
//!   (`refresh_ttl`). Only sent on auth endpoints, never on user/media routes.
//!
//! Both use `SameSite=Lax` — strict would drop the cookie on top-level OAuth
//! redirects from Google back to the callback URL.

use axum_extra::extract::cookie::{Cookie, SameSite};
use time::Duration as TimeDuration;

use crate::config::AppConfig;

pub const ACCESS_COOKIE: &str = "vonk_access";
pub const REFRESH_COOKIE: &str = "vonk_refresh";

const REFRESH_PATH: &str = "/api/auth";

/// Build the access-token cookie.
pub fn access(value: String, cfg: &AppConfig) -> Cookie<'static> {
    let mut c = Cookie::build((ACCESS_COOKIE, value))
        .http_only(true)
        .secure(cfg.environment.is_production())
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(TimeDuration::seconds(cfg.jwt_access_ttl.as_secs() as i64))
        .build();
    if let Some(domain) = cfg.cookie_domain.as_deref() {
        c.set_domain(domain.to_string());
    }
    c
}

/// Build the refresh-token cookie (opaque session UUID).
pub fn refresh(value: String, cfg: &AppConfig) -> Cookie<'static> {
    let mut c = Cookie::build((REFRESH_COOKIE, value))
        .http_only(true)
        .secure(cfg.environment.is_production())
        .same_site(SameSite::Lax)
        .path(REFRESH_PATH)
        .max_age(TimeDuration::seconds(cfg.refresh_ttl.as_secs() as i64))
        .build();
    if let Some(domain) = cfg.cookie_domain.as_deref() {
        c.set_domain(domain.to_string());
    }
    c
}

/// Cookie that clears the access cookie (max-age 0).
pub fn clear_access(cfg: &AppConfig) -> Cookie<'static> {
    let mut c = Cookie::build((ACCESS_COOKIE, ""))
        .http_only(true)
        .secure(cfg.environment.is_production())
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(TimeDuration::seconds(0))
        .build();
    if let Some(domain) = cfg.cookie_domain.as_deref() {
        c.set_domain(domain.to_string());
    }
    c
}

/// Cookie that clears the refresh cookie (max-age 0).
pub fn clear_refresh(cfg: &AppConfig) -> Cookie<'static> {
    let mut c = Cookie::build((REFRESH_COOKIE, ""))
        .http_only(true)
        .secure(cfg.environment.is_production())
        .same_site(SameSite::Lax)
        .path(REFRESH_PATH)
        .max_age(TimeDuration::seconds(0))
        .build();
    if let Some(domain) = cfg.cookie_domain.as_deref() {
        c.set_domain(domain.to_string());
    }
    c
}
