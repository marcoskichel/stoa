//! Fixture: docs that add semantic information beyond the identifier. Must
//! NOT be flagged.

/// Canonical workspace identifier; used as the recall-index primary key.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Active user session, keyed by the bearer token issued at login.
pub struct UserSession {
    pub id: String,
}

/// Returns the bearer token if the request is authenticated, otherwise the
/// anonymous sentinel. Caller must not log the token.
pub fn token() -> &'static str {
    "x"
}
