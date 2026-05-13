//! Build a recall query from recent activity.
//!
//! Combines three signals (per ROADMAP M5): the current directory's
//! basename, the git remote URL (if any), and the stems + H1 titles of
//! the most recently modified wiki pages in the last 24h. Empty
//! workspaces produce an empty query so the BM25 search returns no
//! hits and the relevance gate fires.
//!
//! Token decomposition: wiki stems are stripped of the `ent-`/`con-`/
//! `syn-` page-kind prefix and the rest is split on `-` so an entity
//! like `ent-redis-cache` contributes the searchable tokens
//! `redis cache`. Hidden directory basenames (those starting with `.`,
//! e.g. tempdirs) are dropped — they would only narrow an AND-default
//! BM25 match without contributing a real signal.

use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::workspace::InjectWorkspace;

/// How many recently-modified wiki pages to fold into the query.
const MAX_RECENT_WIKI_PAGES: usize = 8;

/// Recency window for "recently edited" wiki pages.
const RECENT_WINDOW: Duration = Duration::from_hours(24);

/// Construct a fallback ladder of recall queries for `ws` given `cwd`.
///
/// BM25 default is AND, so a single concatenated query that includes
/// every signal (cwd basename + git remote + wiki stems + titles) is
/// the most-specific match — it returns hits only when all tokens
/// co-occur in some doc. The ladder degrades from "all signals" to
/// "wiki stems + titles" to "wiki titles only" to "first wiki stem
/// token only", trying each in order until one returns hits. An
/// empty workspace produces an empty ladder so the relevance gate
/// fires on the very first try.
///
/// Callers should iterate the returned `Vec` and stop on the first
/// non-empty hit set; the *successful* query is what gets audited.
pub(crate) fn build_query_ladder(ws: &InjectWorkspace, cwd: Option<&Path>) -> Vec<String> {
    let cwd_token = cwd_basename_token(cwd);
    let remote = git_remote_url(cwd, &ws.root);
    let recents = recent_wiki_entries(&ws.wiki());
    let stem_toks: Vec<String> = recents.iter().flat_map(|e| stem_tokens(&e.stem)).collect();
    let titles: Vec<String> = recents.iter().filter_map(|e| e.title.clone()).collect();
    candidate_queries(cwd_token.as_deref(), remote.as_deref(), &stem_toks, &titles)
}

fn candidate_queries(
    cwd_token: Option<&str>,
    remote: Option<&str>,
    stem_tokens: &[String],
    titles: &[String],
) -> Vec<String> {
    let mut ladder: Vec<String> = Vec::new();
    push_join(&mut ladder, &full_signal(cwd_token, remote, stem_tokens, titles));
    push_join(&mut ladder, &joined(stem_tokens, titles));
    push_join(&mut ladder, titles);
    if let Some(first) = stem_tokens.first() {
        push_join(&mut ladder, std::slice::from_ref(first));
    }
    dedup_preserving_order(ladder)
}

fn full_signal(
    cwd_token: Option<&str>,
    remote: Option<&str>,
    stem_tokens: &[String],
    titles: &[String],
) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    parts.extend(cwd_token.map(str::to_owned));
    parts.extend(remote.map(str::to_owned));
    parts.extend(stem_tokens.iter().cloned());
    parts.extend(titles.iter().cloned());
    parts
}

fn joined(stem_tokens: &[String], titles: &[String]) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    parts.extend(stem_tokens.iter().cloned());
    parts.extend(titles.iter().cloned());
    parts
}

fn push_join(ladder: &mut Vec<String>, parts: &[String]) {
    let s = collapse_whitespace(&parts.join(" "));
    if !s.is_empty() {
        ladder.push(s);
    }
}

fn dedup_preserving_order(items: Vec<String>) -> Vec<String> {
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut out: Vec<String> = Vec::with_capacity(items.len());
    for item in items {
        if seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

/// Return the basename as a single-element iterator, dropping dotfile
/// names like a tempdir's `.tmpXXX` that would only narrow the AND
/// match without contributing semantic signal.
fn cwd_basename_token(cwd: Option<&Path>) -> Option<String> {
    let name = cwd?.file_name()?.to_str()?;
    if name.starts_with('.') {
        return None;
    }
    Some(name.to_owned())
}

/// Walk up from `start` (or `fallback`) for a `.git/config`, then
/// extract the first `url = ...` value. Best-effort: any failure
/// returns `None` so the caller skips the remote signal.
fn git_remote_url(start: Option<&Path>, fallback: &Path) -> Option<String> {
    let origin = start.unwrap_or(fallback);
    let cfg = find_git_config(origin)?;
    let body = fs::read_to_string(&cfg).ok()?;
    extract_first_url(&body)
}

fn find_git_config(start: &Path) -> Option<PathBuf> {
    let mut here: Option<&Path> = Some(start);
    while let Some(dir) = here {
        let candidate = dir.join(".git").join("config");
        if candidate.is_file() {
            return Some(candidate);
        }
        here = dir.parent();
    }
    None
}

fn extract_first_url(body: &str) -> Option<String> {
    body.lines().map(str::trim).find_map(parse_git_url_line)
}

fn parse_git_url_line(line: &str) -> Option<String> {
    let rest = line.strip_prefix("url")?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_owned())
    }
}

/// Decompose a wiki stem into searchable tokens by stripping the
/// page-kind prefix and splitting the remainder on `-`.
fn stem_tokens(stem: &str) -> Vec<String> {
    let body = strip_page_prefix(stem);
    body.split('-')
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

fn strip_page_prefix(stem: &str) -> &str {
    for prefix in ["ent-", "con-", "syn-"] {
        if let Some(rest) = stem.strip_prefix(prefix) {
            return rest;
        }
    }
    stem
}

#[derive(Debug)]
struct WikiEntry {
    stem: String,
    title: Option<String>,
}

/// Up to N most-recently-modified `*.md` files under `wiki/` in the
/// last 24h, with their H1 title parsed (if any).
fn recent_wiki_entries(wiki_root: &Path) -> Vec<WikiEntry> {
    let raw = collect_recent_wiki(wiki_root);
    raw.into_iter().take(MAX_RECENT_WIKI_PAGES).collect()
}

/// Hard cap on candidate `*.md` files we collect *before* sorting by
/// mtime. Bounds the worst-case walk on a wiki with thousands of pages
/// so the hot path stays sub-millisecond on a cold FS cache.
const MAX_WIKI_CANDIDATES: usize = 256;

fn collect_recent_wiki(wiki_root: &Path) -> Vec<WikiEntry> {
    if !wiki_root.is_dir() {
        return Vec::new();
    }
    let mut accum: Vec<(SystemTime, PathBuf, String)> = Vec::new();
    let cutoff = SystemTime::now()
        .checked_sub(RECENT_WINDOW)
        .unwrap_or(SystemTime::UNIX_EPOCH);
    walk_wiki_md(wiki_root, &mut accum, cutoff);
    accum.sort_by_key(|entry| Reverse(entry.0));
    accum
        .into_iter()
        .take(MAX_RECENT_WIKI_PAGES)
        .map(|(_, p, s)| materialize(&p, s))
        .collect()
}

fn materialize(path: &Path, stem: String) -> WikiEntry {
    WikiEntry {
        stem,
        title: parse_h1_title(path),
    }
}

fn parse_h1_title(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    let body = strip_yaml_frontmatter(&raw);
    body.lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("# ").map(str::trim).map(str::to_owned))
}

fn strip_yaml_frontmatter(raw: &str) -> &str {
    if let Some(rest) = raw.strip_prefix("---\n")
        && let Some(end) = rest.find("\n---")
    {
        let after = &rest[end + 4..];
        return after.strip_prefix('\n').unwrap_or(after);
    }
    raw
}

fn walk_wiki_md(dir: &Path, accum: &mut Vec<(SystemTime, PathBuf, String)>, cutoff: SystemTime) {
    if accum.len() >= MAX_WIKI_CANDIDATES {
        return;
    }
    let Ok(read) = fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        if accum.len() >= MAX_WIKI_CANDIDATES {
            return;
        }
        let path = entry.path();
        if path.is_dir() {
            walk_wiki_md(&path, accum, cutoff);
            continue;
        }
        try_record_md(&path, accum, cutoff);
    }
}

fn try_record_md(path: &Path, accum: &mut Vec<(SystemTime, PathBuf, String)>, cutoff: SystemTime) {
    if path.extension().and_then(|e| e.to_str()) != Some("md") {
        return;
    }
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return;
    };
    if is_meta_page(stem) {
        return;
    }
    let Ok(meta) = fs::metadata(path) else { return };
    let Ok(mtime) = meta.modified() else { return };
    if mtime < cutoff {
        return;
    }
    accum.push((mtime, path.to_path_buf(), stem.to_owned()));
}

/// Skip the workspace's scaffold pages (`index.md`, `log.md`,
/// `lint-report.md`) — they describe the wiki itself, not entity
/// content, and would only narrow the AND-default BM25 match.
fn is_meta_page(stem: &str) -> bool {
    matches!(stem, "index" | "log" | "lint-report")
}

fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}
