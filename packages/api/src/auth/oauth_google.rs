//! Minimal Google OAuth 2.0 / OIDC helpers.
//!
//! We deliberately avoid the [`oauth2`] crate here — its bundled HTTP client
//! feature pins `reqwest 0.11` which conflicts with the rest of the workspace
//! on 0.12. The Google flow is small enough to implement by hand:
//!
//! 1. Generate a random `state` + PKCE pair.
//! 2. Redirect to [`AUTHORIZE_URL`] with appropriate query params.
//! 3. Google redirects back to our callback with `code` + `state`.
//! 4. POST to [`TOKEN_URL`] (form-urlencoded) to exchange code for tokens.
//! 5. GET [`USERINFO_URL`] with the bearer access token for user identity.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::AppConfig;

pub const AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

/// A PKCE challenge/verifier pair.
pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
}

impl PkcePair {
    /// Generate a new S256 PKCE pair per RFC 7636.
    pub fn new_sha256() -> Self {
        let mut buf = [0u8; 64];
        rand::thread_rng().fill_bytes(&mut buf);
        let verifier = URL_SAFE_NO_PAD.encode(buf);

        let digest = Sha256::digest(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(digest);

        PkcePair { verifier, challenge }
    }
}

/// Generate a random URL-safe CSRF / state token.
pub fn random_state() -> String {
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

/// Build the Google authorize URL.
pub fn build_authorize_url(cfg: &AppConfig, state: &str, pkce_challenge: &str) -> String {
    let params = [
        ("response_type", "code"),
        ("client_id", cfg.google_client_id.as_str()),
        ("redirect_uri", &cfg.google_redirect_uri()),
        ("scope", "openid email profile"),
        ("state", state),
        ("code_challenge", pkce_challenge),
        ("code_challenge_method", "S256"),
        ("access_type", "online"),
        ("include_granted_scopes", "true"),
        ("prompt", "select_account"),
    ];
    let qs = params
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{AUTHORIZE_URL}?{qs}")
}

/// Successful token-endpoint response.
///
/// Only `access_token` is read in Phase 1 (enough to hit `/userinfo`). The
/// other fields are kept on the struct so future phases can persist them in
/// `user_auth_providers` without changing the parser.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct GoogleTokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
}

/// Subset of Google's OIDC userinfo response we consume.
///
/// `picture` is deserialized but not used in Phase 1 (we ask the user for
/// their own avatar in onboarding); kept for future auto-import.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleUserInfo {
    pub sub: String,
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: bool,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub picture: Option<String>,
    pub locale: Option<String>,
}

/// Exchange the authorization code for tokens.
pub async fn exchange_code(
    http: &reqwest::Client,
    cfg: &AppConfig,
    code: &str,
    pkce_verifier: &str,
) -> anyhow::Result<GoogleTokenResponse> {
    let form = [
        ("code", code),
        ("client_id", cfg.google_client_id.as_str()),
        ("client_secret", cfg.google_client_secret.as_str()),
        ("redirect_uri", &cfg.google_redirect_uri()),
        ("grant_type", "authorization_code"),
        ("code_verifier", pkce_verifier),
    ];
    let resp = http.post(TOKEN_URL).form(&form).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("google token endpoint {}: {}", status, body);
    }
    let tokens: GoogleTokenResponse = resp.json().await?;
    Ok(tokens)
}

/// Fetch userinfo with a bearer access token.
pub async fn fetch_userinfo(
    http: &reqwest::Client,
    access_token: &str,
) -> anyhow::Result<GoogleUserInfo> {
    let resp = http
        .get(USERINFO_URL)
        .bearer_auth(access_token)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("google userinfo {}: {}", status, body);
    }
    let info: GoogleUserInfo = resp.json().await?;
    Ok(info)
}
