//! Admin-only API endpoints.
//!
//! All routes under `/api/admin/*` require an authenticated user with
//! admin privileges. For Phase 3.5 MVP, "admin" = user with id=1
//! (the first registered user). A proper role system comes later.

pub mod nodes;
