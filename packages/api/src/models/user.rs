//! User row mapping and helpers.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Full user row as stored in the `users` table.
///
/// `id` (BIGSERIAL) is the internal primary key — never leaked in public JSON.
/// `uuid` is the externally-visible identifier. Some fields (e.g. `updated_at`,
/// `is_suspended`) are selected but not surfaced on every response; they're
/// present so one query shape can serve multiple endpoints.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub uuid: Uuid,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub location_city: Option<String>,
    pub location_country: Option<String>,
    pub locale: Option<String>,
    pub is_private: Option<bool>,
    pub is_suspended: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub onboarding_completed_at: Option<DateTime<Utc>>,
}

impl User {
    /// True when the user still needs to complete the onboarding wizard.
    pub fn needs_onboarding(&self) -> bool {
        self.onboarding_completed_at.is_none()
    }

    /// The stable subset of columns we select everywhere.
    ///
    /// Keeping this as a `const &str` avoids drift between queries.
    pub const COLUMNS: &'static str = "id, uuid, username, display_name, email, email_verified, \
                                       bio, avatar_url, banner_url, location_city, location_country, \
                                       locale, is_private, is_suspended, created_at, updated_at, \
                                       onboarding_completed_at";
}

/// Private profile (what the user sees for themselves — includes email).
#[derive(Debug, Serialize)]
pub struct MeProfile {
    pub uuid: Uuid,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub bio: String,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub location_city: Option<String>,
    pub location_country: Option<String>,
    pub locale: String,
    pub is_private: bool,
    pub needs_onboarding: bool,
    pub created_at: DateTime<Utc>,
}

impl From<&User> for MeProfile {
    fn from(u: &User) -> Self {
        MeProfile {
            uuid: u.uuid,
            username: u.username.clone(),
            display_name: u.display_name.clone(),
            email: u.email.clone(),
            email_verified: u.email_verified.unwrap_or(false),
            bio: u.bio.clone().unwrap_or_default(),
            avatar_url: u.avatar_url.clone(),
            banner_url: u.banner_url.clone(),
            location_city: u.location_city.clone(),
            location_country: u.location_country.clone(),
            locale: u.locale.clone().unwrap_or_else(|| "nl".to_string()),
            is_private: u.is_private.unwrap_or(false),
            needs_onboarding: u.needs_onboarding(),
            created_at: u.created_at,
        }
    }
}

/// Public profile (what other users / anonymous visitors see).
/// Strips email, internal id, suspension details.
#[derive(Debug, Serialize)]
pub struct PublicProfile {
    pub uuid: Uuid,
    pub username: String,
    pub display_name: String,
    pub bio: String,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub location_city: Option<String>,
    pub location_country: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<&User> for PublicProfile {
    fn from(u: &User) -> Self {
        PublicProfile {
            uuid: u.uuid,
            username: u.username.clone(),
            display_name: u.display_name.clone(),
            bio: u.bio.clone().unwrap_or_default(),
            avatar_url: u.avatar_url.clone(),
            banner_url: u.banner_url.clone(),
            location_city: u.location_city.clone(),
            location_country: u.location_country.clone(),
            created_at: u.created_at,
        }
    }
}
