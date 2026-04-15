//! Unified API error type.
//!
//! Every handler returns `Result<T, ApiError>`. Conversions from the common
//! infrastructure errors (sqlx, redis, reqwest, anyhow, serde) are provided so
//! handlers can use `?` freely.
//!
//! Wire format matches `CLAUDE.md` API conventions:
//! ```json
//! { "error": { "code": "not_found", "message": "..." } }
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// Errors returned to clients.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not found")]
    NotFound,

    #[error("unauthenticated")]
    Unauthenticated,

    #[error("forbidden")]
    Forbidden,

    #[error("{message}")]
    BadRequest {
        code: &'static str,
        message: String,
    },

    #[error("{message}")]
    Conflict {
        code: &'static str,
        message: String,
    },

    #[error("payload too large")]
    PayloadTooLarge,

    #[error("upstream error")]
    Upstream(#[source] anyhow::Error),

    #[error("internal error")]
    Internal(#[source] anyhow::Error),
}

impl ApiError {
    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::BadRequest {
            code,
            message: message.into(),
        }
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::Conflict {
            code,
            message: message.into(),
        }
    }

    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::Unauthenticated => (StatusCode::UNAUTHORIZED, "unauthenticated"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            Self::BadRequest { code, .. } => (StatusCode::BAD_REQUEST, code),
            Self::Conflict { code, .. } => (StatusCode::CONFLICT, code),
            Self::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large"),
            Self::Upstream(_) => (StatusCode::BAD_GATEWAY, "upstream_error"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        }
    }

    fn public_message(&self) -> String {
        match self {
            Self::NotFound => "not found".into(),
            Self::Unauthenticated => "authentication required".into(),
            Self::Forbidden => "forbidden".into(),
            Self::BadRequest { message, .. } | Self::Conflict { message, .. } => message.clone(),
            Self::PayloadTooLarge => "payload too large".into(),
            // Don't leak internal detail to clients.
            Self::Upstream(_) => "upstream service error".into(),
            Self::Internal(_) => "internal server error".into(),
        }
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: String,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    error: ErrorBody<'a>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        let message = self.public_message();

        if matches!(self, Self::Internal(_) | Self::Upstream(_)) {
            // Walk the full error chain so upstream causes (e.g. the exact
            // S3 / reqwest failure) don't get swallowed by the thiserror
            // Display shim.
            let chain = match &self {
                Self::Upstream(e) | Self::Internal(e) => {
                    let mut parts = vec![e.to_string()];
                    let mut src: Option<&dyn std::error::Error> = e.source();
                    while let Some(s) = src {
                        parts.push(s.to_string());
                        src = s.source();
                    }
                    parts.join(" ← ")
                }
                _ => self.to_string(),
            };
            tracing::error!(error = %self, chain = %chain, ?status, "api error");
        } else {
            tracing::debug!(error = %self, ?status, "api error");
        }

        let body = Json(ErrorEnvelope {
            error: ErrorBody {
                code,
                message,
            },
        });
        (status, body).into_response()
    }
}

// ── Conversions ──────────────────────────────────────────────

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ApiError::NotFound,
            other => ApiError::Internal(other.into()),
        }
    }
}

impl From<redis::RedisError> for ApiError {
    fn from(err: redis::RedisError) -> Self {
        ApiError::Internal(err.into())
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::Upstream(err.into())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::Internal(err.into())
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(errs: validator::ValidationErrors) -> Self {
        ApiError::BadRequest {
            code: "validation_failed",
            message: errs.to_string(),
        }
    }
}

/// Convenient alias for handler results.
pub type ApiResult<T> = Result<T, ApiError>;
