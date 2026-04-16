//! Runtime configuration loaded from environment variables.
//!
//! Loaded once at startup and shared via [`crate::state::AppState`].

use std::time::Duration;

use anyhow::{anyhow, Context, Result};

/// Deployment environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    pub fn is_production(self) -> bool {
        matches!(self, Environment::Production)
    }
}

/// Runtime configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub environment: Environment,

    // URLs
    pub api_url: String,
    pub web_url: String,

    // Database
    pub database_url: String,

    // Cache
    pub redis_url: String,

    // Auth
    pub jwt_secret: String,
    pub jwt_access_ttl: Duration,
    pub refresh_ttl: Duration,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub google_client_id: String,
    pub google_client_secret: String,

    // Object storage (MinIO / S3)
    pub s3_endpoint: String,
    pub s3_region: String,
    pub s3_bucket: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_force_path_style: bool,

    // Privacy
    pub ip_hash_salt: String,

    // Cookies
    pub cookie_domain: Option<String>,
}

impl AppConfig {
    /// Build from the process environment. Returns an error on missing required vars.
    pub fn from_env() -> Result<Self> {
        let environment = match env_or("ENVIRONMENT", "development").to_ascii_lowercase().as_str()
        {
            "production" | "prod" => Environment::Production,
            _ => Environment::Development,
        };

        let jwt_secret = env_required("JWT_SECRET")?;
        if environment.is_production() && jwt_secret.len() < 32 {
            return Err(anyhow!(
                "JWT_SECRET must be at least 32 chars in production (got {})",
                jwt_secret.len()
            ));
        }
        if jwt_secret.starts_with("CHANGE_ME") {
            return Err(anyhow!(
                "JWT_SECRET is still a placeholder; generate a real one"
            ));
        }

        let ip_hash_salt = env_required("IP_HASH_SALT")?;
        if ip_hash_salt.starts_with("CHANGE_ME") {
            return Err(anyhow!(
                "IP_HASH_SALT is still a placeholder; generate a real one"
            ));
        }

        let cookie_domain = std::env::var("COOKIE_DOMAIN")
            .ok()
            .and_then(|s| if s.trim().is_empty() { None } else { Some(s) });

        Ok(Self {
            environment,
            api_url: env_or("API_URL", "http://localhost:8080"),
            web_url: env_or("WEB_URL", "http://localhost:5173"),
            database_url: env_required("DATABASE_URL")?,
            redis_url: env_or("REDIS_URL", "redis://localhost:6380"),
            jwt_secret,
            jwt_access_ttl: Duration::from_secs(env_parsed("JWT_ACCESS_TTL_SECS", 15 * 60)?),
            refresh_ttl: Duration::from_secs(env_parsed("REFRESH_TTL_SECS", 30 * 24 * 60 * 60)?),
            github_client_id: std::env::var("GITHUB_CLIENT_ID").unwrap_or_default(),
            github_client_secret: std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_default(),
            google_client_id: std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            google_client_secret: std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
            s3_endpoint: env_or("S3_ENDPOINT", "http://localhost:9000"),
            s3_region: env_or("S3_REGION", "us-east-1"),
            s3_bucket: env_or("S3_BUCKET", "vonk-media"),
            s3_access_key: env_required("S3_ACCESS_KEY")?,
            s3_secret_key: env_required("S3_SECRET_KEY")?,
            s3_force_path_style: env_or("S3_FORCE_PATH_STYLE", "true")
                .parse()
                .unwrap_or(true),
            ip_hash_salt,
            cookie_domain,
        })
    }

    /// Returns the OAuth redirect URI for the Google callback.
    ///
    /// `origin` overrides the configured `api_url` when present — used to
    /// honour the actual host the user is on (e.g. a login started on
    /// `vonk.social` returns to `vonk.social`, not to a staging host baked
    /// into the server env). Falls back to `api_url` when no host is known.
    pub fn google_redirect_uri(&self, origin: Option<&str>) -> String {
        let base = origin
            .map(|o| o.trim_end_matches('/').to_string())
            .unwrap_or_else(|| self.api_url.trim_end_matches('/').to_string());
        format!("{base}/api/auth/callback/google")
    }

    /// True when Google OAuth credentials are configured.
    pub fn google_configured(&self) -> bool {
        !self.google_client_id.is_empty() && !self.google_client_secret.is_empty()
    }

    /// True when GitHub OAuth credentials are configured.
    pub fn github_configured(&self) -> bool {
        !self.github_client_id.is_empty() && !self.github_client_secret.is_empty()
    }
}

fn env_required(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("missing required env var: {key}"))
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_parsed<T>(key: &str, default: T) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match std::env::var(key) {
        Ok(v) => v
            .parse::<T>()
            .with_context(|| format!("invalid value for {key}")),
        Err(_) => Ok(default),
    }
}
