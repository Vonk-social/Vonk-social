//! ActivityPub JSON-LD types.
//!
//! These map to the ActivityPub / ActivityStreams vocabulary used in
//! federation between Vonk and other fediverse software (Mastodon,
//! Pixelfed, etc.).

use serde::{Deserialize, Serialize};

/// ActivityPub content type used in Accept headers and responses.
pub const AP_CONTENT_TYPE: &str = "application/activity+json";

/// JSON-LD context used on every AP object.
pub const AP_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";

/// Security vocabulary context for `publicKey` on Person actors.
pub const SECURITY_CONTEXT: &str = "https://w3id.org/security/v1";

// ── Actor (Person) ──────────────────────────────────────────

/// An ActivityPub Person object representing a Vonk user.
#[derive(Debug, Serialize)]
pub struct ApPerson {
    #[serde(rename = "@context")]
    pub context: Vec<&'static str>,
    pub id: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
    #[serde(rename = "preferredUsername")]
    pub preferred_username: String,
    pub name: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<ApImage>,
    pub inbox: String,
    pub outbox: String,
    pub followers: String,
    pub following: String,
    #[serde(rename = "publicKey")]
    pub public_key: ApPublicKey,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct ApPublicKey {
    pub id: String,
    pub owner: String,
    #[serde(rename = "publicKeyPem")]
    pub public_key_pem: String,
}

#[derive(Debug, Serialize)]
pub struct ApImage {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub url: String,
    #[serde(rename = "mediaType")]
    pub media_type: &'static str,
}

// ── Collections ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ApOrderedCollection {
    #[serde(rename = "@context")]
    pub context: &'static str,
    pub id: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
    #[serde(rename = "totalItems")]
    pub total_items: i64,
    #[serde(rename = "orderedItems")]
    pub ordered_items: Vec<serde_json::Value>,
}

// ── Activities (inbound) ────────────────────────────────────

/// A generic incoming ActivityPub activity.
#[derive(Debug, Deserialize)]
pub struct IncomingActivity {
    /// Activity ID (may be None for some implementations).
    #[allow(dead_code)]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    pub actor: String,
    pub object: serde_json::Value,
}

// ── WebFinger ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct WebFingerResponse {
    pub subject: String,
    pub links: Vec<WebFingerLink>,
}

#[derive(Debug, Serialize)]
pub struct WebFingerLink {
    pub rel: &'static str,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub href: String,
}

// ── NodeInfo ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct NodeInfoWellKnown {
    pub links: Vec<NodeInfoWellKnownLink>,
}

#[derive(Debug, Serialize)]
pub struct NodeInfoWellKnownLink {
    pub rel: &'static str,
    pub href: String,
}

#[derive(Debug, Serialize)]
pub struct NodeInfo {
    pub version: &'static str,
    pub software: NodeInfoSoftware,
    pub protocols: Vec<&'static str>,
    pub usage: NodeInfoUsage,
    #[serde(rename = "openRegistrations")]
    pub open_registrations: bool,
}

#[derive(Debug, Serialize)]
pub struct NodeInfoSoftware {
    pub name: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Serialize)]
pub struct NodeInfoUsage {
    pub users: NodeInfoUsers,
    #[serde(rename = "localPosts")]
    pub local_posts: i64,
}

#[derive(Debug, Serialize)]
pub struct NodeInfoUsers {
    pub total: i64,
    #[serde(rename = "activeMonth")]
    pub active_month: i64,
    #[serde(rename = "activeHalfyear")]
    pub active_halfyear: i64,
}
