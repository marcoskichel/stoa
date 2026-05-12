//! Page-id parsing + routing (ARCHITECTURE §2).
//!
//! Stoa page ids are slug-style and carry a stable prefix that maps directly
//! to a wiki sub-directory:
//!
//! | Prefix | Directory       |
//! |--------|-----------------|
//! | `ent-` | `wiki/entities/` |
//! | `con-` | `wiki/concepts/` |
//! | `syn-` | `wiki/synthesis/` |
//!
//! This prefix→dir mapping is the only routing rule in the system.

/// One of the three canonical wiki sub-directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageDir {
    /// `wiki/entities/`
    Entities,
    /// `wiki/concepts/`
    Concepts,
    /// `wiki/synthesis/`
    Synthesis,
}

impl PageDir {
    /// Relative sub-directory name under `wiki/`.
    #[must_use]
    pub fn as_subdir(self) -> &'static str {
        match self {
            Self::Entities => "entities",
            Self::Concepts => "concepts",
            Self::Synthesis => "synthesis",
        }
    }

    /// All three directories in canonical order.
    #[must_use]
    pub fn all() -> [Self; 3] {
        [Self::Entities, Self::Concepts, Self::Synthesis]
    }
}

/// Max total id length, including the `xxx-` prefix. Prevents pathologically
/// long filenames on case-sensitive filesystems (most cap at 255 bytes).
const MAX_ID_LEN: usize = 128;

/// A wiki page id with its routing dir resolved.
///
/// Construction is restricted to [`Id::parse`], which enforces the slug
/// grammar `(ent|con|syn)-[a-z0-9]+(-[a-z0-9]+)*` — no `..`, `/`, `\`,
/// uppercase, NUL, or any character outside `[a-z0-9-]`. This makes the
/// id structurally safe to interpolate into a filesystem path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Id {
    /// Full id including the prefix (e.g. `ent-redis`).
    pub raw: String,
    /// Directory derived from the id prefix.
    pub dir: PageDir,
}

impl Id {
    /// Parse an id, classifying by prefix and enforcing the slug grammar.
    /// Returns `None` for unknown prefixes, invalid characters, empty
    /// suffixes, or ids longer than 128 bytes.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        if raw.len() > MAX_ID_LEN {
            return None;
        }
        let (dir, suffix) = Self::split_prefix(raw)?;
        if !is_valid_suffix(suffix) {
            return None;
        }
        Some(Self { raw: raw.to_owned(), dir })
    }

    /// Map an id prefix to its canonical [`PageDir`]. Does **not** validate
    /// the suffix — use [`Id::parse`] for full validation.
    #[must_use]
    pub fn dir_for(raw: &str) -> Option<PageDir> {
        Self::split_prefix(raw).map(|(dir, _)| dir)
    }

    fn split_prefix(raw: &str) -> Option<(PageDir, &str)> {
        raw.strip_prefix("ent-")
            .map(|s| (PageDir::Entities, s))
            .or_else(|| raw.strip_prefix("con-").map(|s| (PageDir::Concepts, s)))
            .or_else(|| raw.strip_prefix("syn-").map(|s| (PageDir::Synthesis, s)))
    }
}

/// Validate `suffix` matches `[a-z0-9]+(-[a-z0-9]+)*` — non-empty, no
/// leading/trailing/consecutive hyphens, ASCII lowercase + digits only.
fn is_valid_suffix(suffix: &str) -> bool {
    if suffix.is_empty() || suffix.starts_with('-') || suffix.ends_with('-') {
        return false;
    }
    let mut prev_hyphen = false;
    for c in suffix.chars() {
        let ok = c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
        if !ok || (c == '-' && prev_hyphen) {
            return false;
        }
        prev_hyphen = c == '-';
    }
    true
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

/// Max session-id length, matching [`MAX_ID_LEN`] for [`Id`]. Keeps the
/// `sessions/<id>.jsonl` path well under the typical 255-byte filename cap.
const MAX_SESSION_ID_LEN: usize = 128;

/// A capture-pipeline session id (Claude Code / Cursor / etc).
///
/// Session ids come from upstream agent platforms (UUIDs, slugs, hex
/// strings, etc.) so we use a wider grammar than [`Id`]: ASCII alnum plus
/// `-`, `_`, `.` — but never `..`, `/`, `\`, NUL, leading `.`, or any
/// non-ASCII byte. This makes a parsed [`SessionId`] structurally safe to
/// interpolate into a filesystem path (e.g. `sessions/<id>.jsonl`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId {
    /// Full id as supplied by the agent platform.
    pub raw: String,
}

impl SessionId {
    /// Parse a session id. Returns `None` for ids that would be unsafe to
    /// use as a path component: empty, longer than 128 bytes, contain `/`,
    /// `\`, `..`, NUL, leading `.`, or any non-ASCII byte.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        if raw.is_empty() || raw.len() > MAX_SESSION_ID_LEN {
            return None;
        }
        if raw.starts_with('.') || raw.contains("..") {
            return None;
        }
        if !raw.chars().all(is_session_char) {
            return None;
        }
        Some(Self { raw: raw.to_owned() })
    }
}

fn is_session_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::{Id, PageDir, SessionId};

    #[test]
    fn parses_entity_prefix() {
        let id = Id::parse("ent-redis").unwrap();
        assert_eq!(id.dir, PageDir::Entities);
    }

    #[test]
    fn parses_concept_prefix() {
        let id = Id::parse("con-rag").unwrap();
        assert_eq!(id.dir, PageDir::Concepts);
    }

    #[test]
    fn parses_synthesis_prefix() {
        let id = Id::parse("syn-x").unwrap();
        assert_eq!(id.dir, PageDir::Synthesis);
    }

    #[test]
    fn rejects_unknown_prefix() {
        assert!(Id::parse("xxx-broken").is_none());
    }

    #[test]
    fn rejects_empty_suffix() {
        assert!(Id::parse("ent-").is_none());
        assert!(Id::parse("con-").is_none());
        assert!(Id::parse("syn-").is_none());
    }

    #[test]
    fn rejects_path_traversal_segments() {
        assert!(Id::parse("ent-..").is_none());
        assert!(Id::parse("ent-../../etc/passwd").is_none());
        assert!(Id::parse("ent-/abs/path").is_none());
        assert!(Id::parse("ent-..\\..\\etc").is_none());
        assert!(Id::parse("ent-foo/bar").is_none());
    }

    #[test]
    fn rejects_uppercase_and_unicode() {
        assert!(Id::parse("ent-Redis").is_none());
        assert!(Id::parse("ent-café").is_none());
        assert!(Id::parse("ent-foo\0bar").is_none());
    }

    #[test]
    fn rejects_hyphen_edges_and_doubles() {
        assert!(Id::parse("ent--foo").is_none());
        assert!(Id::parse("ent-foo-").is_none());
        assert!(Id::parse("ent-foo--bar").is_none());
    }

    #[test]
    fn rejects_overlong_id() {
        let long_suffix: String = "a".repeat(200);
        let raw = format!("ent-{long_suffix}");
        assert!(Id::parse(&raw).is_none());
    }

    #[test]
    fn accepts_multi_segment_slug() {
        let id = Id::parse("syn-redis-vs-memcached").unwrap();
        assert_eq!(id.dir, PageDir::Synthesis);
        assert_eq!(id.raw, "syn-redis-vs-memcached");
    }

    #[test]
    fn accepts_digits_and_mixed() {
        assert!(Id::parse("ent-redis-7").is_some());
        assert!(Id::parse("con-rfc-3339").is_some());
    }

    #[test]
    fn session_id_accepts_realistic_shapes() {
        assert!(SessionId::parse("sess-001").is_some());
        assert!(SessionId::parse("sess-A").is_some());
        assert!(SessionId::parse("550e8400-e29b-41d4-a716-446655440000").is_some());
        assert!(SessionId::parse("claude_2025_05_12").is_some());
        assert!(SessionId::parse("Session.001").is_some());
    }

    #[test]
    fn session_id_rejects_path_traversal() {
        assert!(SessionId::parse("../../etc/foo").is_none());
        assert!(SessionId::parse("..").is_none());
        assert!(SessionId::parse("foo..bar").is_none());
        assert!(SessionId::parse("/abs/path").is_none());
        assert!(SessionId::parse("a/b").is_none());
        assert!(SessionId::parse("a\\b").is_none());
    }

    #[test]
    fn session_id_rejects_hidden_and_empty() {
        assert!(SessionId::parse("").is_none());
        assert!(SessionId::parse(".hidden").is_none());
    }

    #[test]
    fn session_id_rejects_non_ascii_and_nul() {
        assert!(SessionId::parse("sess-café").is_none());
        assert!(SessionId::parse("sess\0bar").is_none());
    }

    #[test]
    fn session_id_rejects_overlong() {
        let raw = "a".repeat(200);
        assert!(SessionId::parse(&raw).is_none());
    }
}
