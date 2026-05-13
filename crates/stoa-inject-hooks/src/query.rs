//! Build a recall query from session and prompt signals.
//!
//! For `UserPromptSubmit`, the user's prompt text is the primary query;
//! workspace signals are folded in only when the prompt is empty. For
//! `SessionStart`, fall back to the cwd basename + git remote + most
//! recently edited wiki pages — the original M5 ladder.

use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::payload::HookEvent;
use crate::workspace::InjectWorkspace;

const MAX_RECENT_WIKI_PAGES: usize = 8;
const RECENT_WINDOW: Duration = Duration::from_hours(24);
const MAX_WIKI_CANDIDATES: usize = 256;

/// Construct an ordered list of queries to try against the daemon.
///
/// The caller iterates this in order, stopping on the first non-empty
/// result. For `UserPromptSubmit` the prompt text alone usually wins;
/// the workspace signals are appended as a fallback ladder when the
/// prompt is empty (a degenerate `/clear`-style submission).
pub(crate) fn build_query_ladder(
    ws: &InjectWorkspace,
    cwd: Option<&Path>,
    event: HookEvent,
    prompt: Option<&str>,
) -> Vec<String> {
    let cwd_token = cwd_basename_token(cwd);
    let remote = git_remote_url(cwd, &ws.root);
    let recents = recent_wiki_entries(&ws.wiki());
    let stem_toks: Vec<String> = recents.iter().flat_map(|e| stem_tokens(&e.stem)).collect();
    let titles: Vec<String> = recents.iter().filter_map(|e| e.title.clone()).collect();

    let mut ladder: Vec<String> = Vec::new();

    if event == HookEvent::UserPromptSubmit
        && let Some(p) = prompt
        && !p.trim().is_empty()
    {
        push_join(&mut ladder, &[p.to_owned()]);
    }

    push_join(
        &mut ladder,
        &full_signal(cwd_token.as_deref(), remote.as_deref(), &stem_toks, &titles),
    );
    push_join(&mut ladder, &joined(&stem_toks, &titles));
    push_join(&mut ladder, &titles);
    if let Some(first) = stem_toks.first() {
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

fn cwd_basename_token(cwd: Option<&Path>) -> Option<String> {
    let name = cwd?.file_name()?.to_str()?;
    if name.starts_with('.') {
        return None;
    }
    Some(name.to_owned())
}

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

fn recent_wiki_entries(wiki_root: &Path) -> Vec<WikiEntry> {
    let raw = collect_recent_wiki(wiki_root);
    raw.into_iter().take(MAX_RECENT_WIKI_PAGES).collect()
}

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

fn is_meta_page(stem: &str) -> bool {
    matches!(stem, "index" | "log" | "lint-report")
}

fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}
