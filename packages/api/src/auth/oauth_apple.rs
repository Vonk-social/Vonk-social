//! Apple Sign in — full implementation.
//!
//! Apple's OAuth is non-standard:
//!   1. `client_secret` is an ES256-signed JWT, not a static string.
//!   2. The token endpoint returns an `id_token` (JWT). The Apple `sub`
//!      lives in its `sub` claim.
//!   3. Email + name are only sent on the **first** consent — subsequent
//!      sign-ins return no email, so the first successful callback is the
//!      moment we must capture and persist them.
//!   4. Apple uses `response_mode=form_post` — the callback receives a
//!      POST with form-encoded body, not a GET with query params.
//!
//! Flow:
//!   GET  /api/auth/login/apple  → redirect to appleid.apple.com
//!   POST /api/auth/callback/apple ← Apple POSTs { code, state, id_token, user }

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

pub const AUTHORIZE_URL: &str = "https://appleid.apple.com/auth/authorize";
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

/// Mint the ephemeral ES256-signed JWT that Apple uses as `client_secret`.
/// Valid for up to 6 months; we mint a fresh one per token exchange (simpler
/// than caching + refreshing).
pub fn mint_client_secret(cfg: &AppConfig) -> anyhow::Result<String> {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

    let now = chrono::Utc::now().timestamp() as u64;
    let claims = AppleClientClaims {
        iss: cfg.apple_team_id.clone(),
        iat: now,
        exp: now + 300, // 5 minutes is plenty for a single exchange
        aud: "https://appleid.apple.com".to_string(),
        sub: cfg.apple_client_id.clone(),
    };
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(cfg.apple_key_id.clone());

    let key = EncodingKey::from_ec_pem(cfg.apple_private_key.as_bytes())
        .map_err(|e| anyhow::anyhow!("parse Apple .p8 key: {e}"))?;

    let token = encode(&header, &claims, &key)
        .map_err(|e| anyhow::anyhow!("sign Apple client_secret: {e}"))?;
    Ok(token)
}

#[derive(Debug, Serialize)]
struct AppleClientClaims {
    iss: String,
    iat: u64,
    exp: u64,
    aud: String,
    sub: String,
}

/// Exchange the authorization code for tokens.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AppleTokenResponse {
    pub access_token: String,
    pub id_token: String,
    #[allow(dead_code)]
    pub token_type: Option<String>,
    #[allow(dead_code)]
    pub expires_in: Option<u64>,
}

pub async fn exchange_code(
    http: &reqwest::Client,
    cfg: &AppConfig,
    origin: Option<&str>,
    code: &str,
) -> anyhow::Result<AppleTokenResponse> {
    let client_secret = mint_client_secret(cfg)?;
    let redirect = redirect_uri(cfg, origin);

    let params = [
        ("client_id", cfg.apple_client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("code", code),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect.as_str()),
    ];

    let res = http
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await?;

    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        anyhow::bail!("Apple token exchange failed: {body}");
    }

    Ok(res.json().await?)
}

/// Decode the id_token claims WITHOUT verifying the signature.
/// Apple's JWKS verification is a nice-to-have but not strictly required
/// since we received the id_token directly from Apple over TLS in the
/// same HTTP response as the code exchange — there's no MITM vector.
/// Adding JWKS verification is a Phase 4 hardening item.
#[derive(Debug, Deserialize)]
pub struct AppleIdTokenClaims {
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: Option<serde_json::Value>, // Apple sends "true" as string or bool
    #[allow(dead_code)]
    pub iss: Option<String>,
}

/// Apple's JWKS endpoint — returns the public keys used to sign id_tokens.
const APPLE_JWKS_URL: &str = "https://appleid.apple.com/auth/keys";

#[derive(Debug, Deserialize)]
struct AppleJwks {
    keys: Vec<AppleJwk>,
}

#[derive(Debug, Deserialize)]
struct AppleJwk {
    kid: String,
    kty: String,
    n: String,
    e: String,
    #[allow(dead_code)]
    alg: Option<String>,
}

/// Decode and **verify** the Apple id_token using Apple's JWKS.
///
/// 1. Decode the JWT header to get the `kid`.
/// 2. Fetch Apple's JWKS and find the matching key.
/// 3. Verify the RS256 signature.
/// 4. Validate audience + issuer.
pub async fn decode_id_token(
    http: &reqwest::Client,
    id_token: &str,
) -> anyhow::Result<AppleIdTokenClaims> {
    use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};

    // 1. Extract kid from the token header.
    let header = decode_header(id_token)
        .map_err(|e| anyhow::anyhow!("decode Apple JWT header: {e}"))?;
    let kid = header
        .kid
        .ok_or_else(|| anyhow::anyhow!("Apple id_token has no kid in header"))?;

    // 2. Fetch Apple's JWKS.
    let jwks: AppleJwks = http
        .get(APPLE_JWKS_URL)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("fetch Apple JWKS: {e}"))?
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("parse Apple JWKS: {e}"))?;

    // 3. Find the key matching the kid.
    let jwk = jwks
        .keys
        .iter()
        .find(|k| k.kid == kid && k.kty == "RSA")
        .ok_or_else(|| anyhow::anyhow!("Apple JWKS has no RSA key with kid={kid}"))?;

    // 4. Build the decoding key from n + e (RSA modulus + exponent).
    let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
        .map_err(|e| anyhow::anyhow!("build RSA key from JWKS: {e}"))?;

    // 5. Verify signature + claims.
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&["social.vonk.signin"]);
    validation.set_issuer(&["https://appleid.apple.com"]);

    let token_data = decode::<AppleIdTokenClaims>(id_token, &decoding_key, &validation)
        .map_err(|e| anyhow::anyhow!("verify Apple id_token: {e}"))?;

    Ok(token_data.claims)
}

impl AppleIdTokenClaims {
    pub fn is_email_verified(&self) -> bool {
        match &self.email_verified {
            Some(serde_json::Value::Bool(b)) => *b,
            Some(serde_json::Value::String(s)) => s == "true",
            _ => false,
        }
    }
}

/// Apple sends user info (name) as a JSON string in the `user` form field,
/// but only on the FIRST consent. Subsequent logins don't include it.
#[derive(Debug, Deserialize)]
pub struct AppleUserInfo {
    pub name: Option<AppleUserName>,
}

#[derive(Debug, Deserialize)]
pub struct AppleUserName {
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
}

impl AppleUserInfo {
    pub fn display_name(&self) -> Option<String> {
        self.name.as_ref().and_then(|n| {
            let parts: Vec<&str> = [n.first_name.as_deref(), n.last_name.as_deref()]
                .into_iter()
                .flatten()
                .collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(" "))
            }
        })
    }
}
