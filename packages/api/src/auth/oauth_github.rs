//! GitHub OAuth 2.0 helpers.
//!
//! Different from Google in two ways:
//!   - GitHub is OAuth 2.0 not OIDC, so we pull the user profile from
//!     `GET /user` + `GET /user/emails` after the token exchange.
//!   - PKCE is optional on GitHub's side, but we use it anyway for
//!     defence in depth.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::AppConfig;

pub const AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
pub const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
pub const USER_URL: &str = "https://api.github.com/user";
pub const EMAILS_URL: &str = "https://api.github.com/user/emails";

pub struct PkcePair {
    pub verifier: String,
    pub challenge: String,
}

impl PkcePair {
    pub fn new_sha256() -> Self {
        let mut buf = [0u8; 64];
        rand::thread_rng().fill_bytes(&mut buf);
        let verifier = URL_SAFE_NO_PAD.encode(buf);
        let digest = Sha256::digest(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(digest);
        PkcePair { verifier, challenge }
    }
}

pub fn random_state() -> String {
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

pub fn callback_path() -> &'static str {
    "/api/auth/callback/github"
}

pub fn redirect_uri(cfg: &AppConfig, origin: Option<&str>) -> String {
    let base = origin
        .map(|o| o.trim_end_matches('/').to_string())
        .unwrap_or_else(|| cfg.api_url.trim_end_matches('/').to_string());
    format!("{base}{}", callback_path())
}

pub fn build_authorize_url(
    cfg: &AppConfig,
    origin: Option<&str>,
    state: &str,
    pkce_challenge: &str,
) -> String {
    let redirect = redirect_uri(cfg, origin);
    let params = [
        ("client_id", cfg.github_client_id.as_str()),
        ("redirect_uri", redirect.as_str()),
        ("scope", "read:user user:email"),
        ("state", state),
        ("code_challenge", pkce_challenge),
        ("code_challenge_method", "S256"),
        ("allow_signup", "true"),
    ];
    let qs = params
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{AUTHORIZE_URL}?{qs}")
}

#[derive(Debug, Deserialize)]
pub struct GitHubTokenResponse {
    pub access_token: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub token_type: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub scope: Option<String>,
}

pub async fn exchange_code(
    http: &reqwest::Client,
    cfg: &AppConfig,
    origin: Option<&str>,
    code: &str,
    pkce_verifier: &str,
) -> anyhow::Result<GitHubTokenResponse> {
    let redirect = redirect_uri(cfg, origin);
    let form = [
        ("code", code),
        ("client_id", cfg.github_client_id.as_str()),
        ("client_secret", cfg.github_client_secret.as_str()),
        ("redirect_uri", redirect.as_str()),
        ("code_verifier", pkce_verifier),
    ];
    let resp = http
        .post(TOKEN_URL)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&form)
        .send()
        .await?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("github token endpoint: {body}");
    }
    Ok(resp.json().await?)
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubUser {
    /// Numeric user ID. We store this as `provider_uid`.
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    #[allow(dead_code)]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

pub async fn fetch_user(http: &reqwest::Client, access_token: &str) -> anyhow::Result<GitHubUser> {
    let mut user: GitHubUser = http
        .get(USER_URL)
        .bearer_auth(access_token)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header(reqwest::header::USER_AGENT, "vonk-api")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // GitHub returns `email: null` for users who hide their email in
    // profile settings. Hit /user/emails to find the primary + verified
    // address — requires the `user:email` scope we asked for.
    if user.email.is_none() {
        let emails: Vec<GitHubEmail> = http
            .get(EMAILS_URL)
            .bearer_auth(access_token)
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .header(reqwest::header::USER_AGENT, "vonk-api")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        user.email = emails
            .into_iter()
            .find(|e| e.primary && e.verified)
            .map(|e| e.email);
    }
    Ok(user)
}
