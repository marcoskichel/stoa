//! On-disk wiki page format: `---\n<yaml>\n---\n<body>`.
//!
//! Shared by `read` (parse) and `write` (build + write). Frontmatter parsing
//! is intentionally permissive on read — we accept whatever YAML is present
//! so existing pages aren't blocked by stricter validation.

use anyhow::{Context, anyhow};

/// Parsed page split into the raw YAML frontmatter block and the body.
#[derive(Debug, Clone)]
pub(crate) struct ParsedPage {
    /// Raw YAML text between the leading `---` delimiters (no fences).
    pub(crate) frontmatter_yaml: String,
    /// Body text after the closing `---` (verbatim, including trailing nl).
    pub(crate) body: String,
}

/// Split a page string into its frontmatter + body halves.
///
/// `path` is only used for error messages — pass the relative path so the
/// user can tell which file is broken.
pub(crate) fn split_page(text: &str, path: &str) -> anyhow::Result<ParsedPage> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
        .ok_or_else(|| anyhow!("page `{path}` is missing leading `---` frontmatter delim"))?;
    let end = find_closing_delim(rest)
        .ok_or_else(|| anyhow!("page `{path}` is missing closing `---` frontmatter delim"))?;
    let (fm, after) = rest.split_at(end);
    let body = trim_after_delim(after).context("computing body slice")?;
    Ok(ParsedPage {
        frontmatter_yaml: fm.to_owned(),
        body: body.to_owned(),
    })
}

fn find_closing_delim(rest: &str) -> Option<usize> {
    // NOTE: only an exact `---` line (LF or CRLF) closes the frontmatter — substrings shouldn't.
    let bytes = rest.as_bytes();
    let mut i = 0_usize;
    while i < bytes.len() {
        let line_end = bytes[i..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(bytes.len(), |o| i + o);
        let line = &rest[i..line_end];
        let stripped = line.strip_suffix('\r').unwrap_or(line);
        if stripped == "---" {
            return Some(i);
        }
        i = line_end + 1;
    }
    None
}

fn trim_after_delim(after: &str) -> anyhow::Result<&str> {
    let rest = after
        .strip_prefix("---\n")
        .or_else(|| after.strip_prefix("---\r\n"))
        .or_else(|| after.strip_prefix("---"))
        .ok_or_else(|| anyhow!("internal: closing delim mis-detected"))?;
    Ok(rest)
}

/// Render a frontmatter YAML block + body into the canonical page format.
#[must_use]
pub(crate) fn render_page(frontmatter_yaml: &str, body: &str) -> String {
    let mut out = String::with_capacity(frontmatter_yaml.len() + body.len() + 16);
    out.push_str("---\n");
    if !frontmatter_yaml.is_empty() && !frontmatter_yaml.ends_with('\n') {
        out.push_str(frontmatter_yaml);
        out.push('\n');
    } else {
        out.push_str(frontmatter_yaml);
    }
    out.push_str("---\n");
    out.push_str(body);
    if !body.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{render_page, split_page};

    #[test]
    fn splits_minimal_page() {
        let text = "---\nid: ent-x\n---\nbody\n";
        let parsed = split_page(text, "x").unwrap();
        assert_eq!(parsed.frontmatter_yaml, "id: ent-x\n");
        assert_eq!(parsed.body, "body\n");
    }

    #[test]
    fn round_trip_preserves_content() {
        let fm = "id: ent-x\nkind: entity\n";
        let body = "Hello.\n";
        let page = render_page(fm, body);
        let parsed = split_page(&page, "x").unwrap();
        assert_eq!(parsed.body, body);
        assert!(parsed.frontmatter_yaml.contains("id: ent-x"));
    }

    #[test]
    fn rejects_no_frontmatter() {
        let err = split_page("hello\n", "x").unwrap_err();
        assert!(err.to_string().contains("leading"));
    }
}
