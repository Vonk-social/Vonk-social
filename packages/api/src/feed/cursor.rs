//! Opaque cursor encoding a `(created_at, id)` tuple.
//!
//! Encoding: URL-safe base64 of JSON `{ts:"RFC3339", id:<i64>}`.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cursor {
    pub ts: DateTime<Utc>,
    pub id: i64,
}

impl Cursor {
    pub fn encode(&self) -> String {
        let json = serde_json::to_vec(self).expect("cursor encodes");
        URL_SAFE_NO_PAD.encode(json)
    }

    pub fn decode(s: &str) -> Option<Self> {
        let bytes = URL_SAFE_NO_PAD.decode(s).ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}
