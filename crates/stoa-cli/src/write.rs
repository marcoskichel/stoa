//! `stoa write <id> [--frontmatter <path>] [--body <path>]`
//!
//! Create or update a wiki page. Auto-fills `id`, `created`, `updated`.
//! Updates `wiki/index.md` and appends an event to `wiki/log.md`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_yaml::{Mapping, Value};

use crate::catalog;
use crate::page::{render_page, split_page};
use crate::workspace::Workspace;

/// Run `stoa write <id> ...` from the current working directory.
pub(crate) fn run(
    id: &str,
    fm_path: Option<&Path>,
    body_path: Option<&Path>,
) -> anyhow::Result<()> {
    let ws = Workspace::current()?;
    let page_path = ws.page_path(id)?;
    let now = Utc::now();
    let existing = read_existing(&page_path)?;
    let frontmatter = build_frontmatter(id, &existing, fm_path, now)?;
    let body = resolve_body(&existing, body_path)?;
    write_page(&page_path, &frontmatter, &body)?;
    append_log(&ws, id, now)?;
    catalog::refresh_index(&ws)?;
    Ok(())
}

struct Existing {
    frontmatter: Mapping,
    body: String,
}

fn read_existing(path: &Path) -> anyhow::Result<Existing> {
    if !path.is_file() {
        return Ok(Existing {
            frontmatter: Mapping::new(),
            body: String::new(),
        });
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading existing page `{}`", path.display()))?;
    let parsed = split_page(&raw, &path.display().to_string())?;
    let frontmatter: Mapping = if parsed.frontmatter_yaml.trim().is_empty() {
        Mapping::new()
    } else {
        serde_yaml::from_str(&parsed.frontmatter_yaml)
            .with_context(|| format!("parsing existing frontmatter in `{}`", path.display()))?
    };
    Ok(Existing { frontmatter, body: parsed.body })
}

fn build_frontmatter(
    id: &str,
    existing: &Existing,
    fm_path: Option<&Path>,
    now: DateTime<Utc>,
) -> anyhow::Result<String> {
    let mut map = match fm_path {
        Some(p) => load_user_frontmatter(p)?,
        None => existing.frontmatter.clone(),
    };
    set_str(&mut map, "id", id);
    let created = pick_created(&existing.frontmatter, now);
    set_str(&mut map, "created", &format_ts(created));
    set_str(&mut map, "updated", &format_ts(now));
    let yaml =
        serde_yaml::to_string(&Value::Mapping(map)).context("serializing computed frontmatter")?;
    Ok(yaml)
}

fn load_user_frontmatter(path: &Path) -> anyhow::Result<Mapping> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading frontmatter file `{}`", path.display()))?;
    let value: Value = serde_yaml::from_str(&text)
        .with_context(|| format!("parsing frontmatter file `{}`", path.display()))?;
    match value {
        Value::Mapping(m) => Ok(m),
        Value::Null => Ok(Mapping::new()),
        _ => Err(anyhow!("frontmatter file `{}` is not a YAML mapping", path.display())),
    }
}

fn pick_created(existing: &Mapping, now: DateTime<Utc>) -> DateTime<Utc> {
    existing
        .get(Value::String("created".to_owned()))
        .and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map_or(now, |t| t.with_timezone(&Utc))
}

fn set_str(map: &mut Mapping, key: &str, value: &str) {
    let _previous = map.insert(Value::String(key.to_owned()), Value::String(value.to_owned()));
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn resolve_body(existing: &Existing, body_path: Option<&Path>) -> anyhow::Result<String> {
    match body_path {
        Some(p) => {
            let text = fs::read_to_string(p)
                .with_context(|| format!("reading body file `{}`", p.display()))?;
            Ok(text)
        },
        None => Ok(existing.body.clone()),
    }
}

fn write_page(path: &Path, frontmatter: &str, body: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating parent of `{}`", path.display()))?;
    }
    let page = render_page(frontmatter, body);
    fs::write(path, page).with_context(|| format!("writing page `{}`", path.display()))?;
    Ok(())
}

fn append_log(ws: &Workspace, id: &str, now: DateTime<Utc>) -> anyhow::Result<()> {
    let log_path: PathBuf = ws.log_md();
    let line = format!("{}  write  {}\n", format_ts(now), id);
    let mut current = fs::read_to_string(&log_path).unwrap_or_default();
    if !current.is_empty() && !current.ends_with('\n') {
        current.push('\n');
    }
    current.push_str(&line);
    fs::write(&log_path, current).with_context(|| format!("appending to `{}`", log_path.display()))
}
