//! Error types for stoa-core.
//!
//! Validation errors carry the offending page id + field so callers can
//! produce actionable diagnostics (see CLI `schema --check`).

use thiserror::Error;

/// Library error for stoa-core. Currently only validation surfaces here;
/// frontmatter parse failures bubble up as `serde_yaml::Error` directly.
#[derive(Debug, Error)]
pub enum Error {
    /// YAML (de)serialization failure.
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// One or more validation rules failed against the workspace schema.
    #[error("validation failed:\n{0}")]
    Validation(String),
}

/// Convenience `Result` alias.
pub type Result<T> = std::result::Result<T, Error>;

/// A single validation failure — produced by [`crate::validate_page`].
///
/// Multiple errors are joined for display in [`Error::Validation`]; this
/// struct exists so callers can render diagnostics in their own format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Page id (e.g. `ent-redis`) the error applies to.
    pub page_id: String,
    /// Field name the rule applies to (e.g. `kind`, `status`, `type`).
    pub field: String,
    /// Human-readable explanation, includes the offending value.
    pub message: String,
}

impl ValidationError {
    /// Construct a new [`ValidationError`].
    pub fn new(
        page_id: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            page_id: page_id.into(),
            field: field.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "page `{}` field `{}`: {}", self.page_id, self.field, self.message,)
    }
}
