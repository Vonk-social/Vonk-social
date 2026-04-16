//! Authentication building blocks: JWT, cookies, OAuth2 providers, middleware.

pub mod cookies;
pub mod ip;
pub mod jwt;
pub mod middleware;
pub mod oauth_github;
pub mod oauth_google;

pub use middleware::AuthUser;
