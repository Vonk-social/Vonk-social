//! ActivityPub protocol endpoints.
//!
//! - `GET  /ap/users/{username}`         — Actor (Person) document
//! - `GET  /ap/users/{username}/outbox`  — Public posts as OrderedCollection
//! - `POST /ap/users/{username}/inbox`   — Receives activities from remote servers

pub mod actor;
pub mod inbox;
pub mod outbox;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(actor::router())
        .merge(outbox::router())
        .merge(inbox::router())
}
