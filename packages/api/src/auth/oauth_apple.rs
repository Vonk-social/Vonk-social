//! Apple Sign in scaffolding.
//!
//! Apple OIDC is meaningfully different from Google and GitHub:
//!
//! 1. `client_secret` is an **ES256-signed JWT** — not a static string. It
//!    must be (re-)minted from a `.p8` private key + team_id + key_id and
//!    expires after ≤6 months.
//! 2. The token endpoint returns an `id_token` (JWT); the Apple `sub`
//!    lives in its `sub` claim, not a `/userinfo` call.
//! 3. Email is only disclosed on **first** consent — later sign-ins return
//!    no email, so the first successful sign-in is the moment we must
//!    capture and store it.
//!
//! This module wires the authorize URL + config plumbing. The token
//! exchange is intentionally left as TODO until we have the Apple
//! developer account + .p8 key on hand. `crate::config::AppConfig::
//! apple_configured()` returns false until then, so the HTTP handler
//! replies with a clean `apple_not_configured` error and no unfinished
//! code path is ever hit.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rand::RngCore;

use crate::config::AppConfig;

pub const AUTHORIZE_URL: &str = "https://appleid.apple.com/auth/authorize";
#[allow(dead_code)]
pub const TOKEN_URL: &str = "https://appleid.apple.com/auth/token";

pub fn random_state() -> String {
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

pub fn callback_path() -> &'static str {
    "/api/auth/callback/apple"
}

pub fn redirect_uri(cfg: &AppConfig, origin: Option<&str>) -> String {
    let base = origin
        .map(|o| o.trim_end_matches('/').to_string())
        .unwrap_or_else(|| cfg.api_url.trim_end_matches('/').to_string());
    format!("{base}{}", callback_path())
}

pub fn build_authorize_url(cfg: &AppConfig, origin: Option<&str>, state: &str) -> String {
    let redirect = redirect_uri(cfg, origin);
    // `response_mode=form_post` is Apple's required mode when requesting
    // `email` scope. The callback will need to accept POST, not GET — we
    // intentionally defer that wiring until creds exist.
    let params = [
        ("client_id", cfg.apple_client_id.as_str()),
        ("redirect_uri", redirect.as_str()),
        ("response_type", "code id_token"),
        ("response_mode", "form_post"),
        ("scope", "name email"),
        ("state", state),
    ];
    let qs = params
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{AUTHORIZE_URL}?{qs}")
}
