//! `stoa schema [--check]` — print `STOA.md` or validate the wiki against it.

use std::fs;
use std::path::Path;

use anyhow::{Context, anyhow};
use stoa_core::{Schema, ValidationError};

use crate::page::split_page;
use crate::workspace::Workspace;

/// Run `stoa schema` (`check=false`) or `stoa schema --check`.
pub(crate) fn run(check: bool) -> anyhow::Result<()> {
    let ws = Workspace::current()?;
    if check {
        run_check(&ws)
    } else {
        run_print(&ws)
    }
}

fn run_print(ws: &Workspace) -> anyhow::Result<()> {
    let text = fs::read_to_string(ws.stoa_md())
        .with_context(|| format!("reading `{}`", ws.stoa_md().display()))?;
    print(&text);
    Ok(())
}

fn run_check(ws: &Workspace) -> anyhow::Result<()> {
    let schema = ws.schema()?;
    let mut errors: Vec<ValidationError> = Vec::new();
    for dir in stoa_core::PageDir::all() {
        let sub = ws.wiki_subdir(dir);
        if !sub.is_dir() {
            continue;
        }
        collect_errors_in(&sub, &schema, &mut errors)?;
    }
    if errors.is_empty() {
        return Ok(());
    }
    let joined = errors
        .iter()
        .map(ValidationError::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    Err(anyhow!("schema check failed:\n{joined}"))
}

fn collect_errors_in(
    dir: &Path,
    schema: &Schema,
    out: &mut Vec<ValidationError>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading dir `{}`", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading `{}`", path.display()))?;
        validate_one(&path, &text, schema, out);
    }
    Ok(())
}

fn validate_one(path: &Path, text: &str, schema: &Schema, out: &mut Vec<ValidationError>) {
    let path_id = page_id_from_path(path);
    match split_page(text, path_id.as_str()) {
        Ok(parsed) => {
            let errs = stoa_core::validate_page(&parsed.frontmatter_yaml, &path_id, schema);
            out.extend(errs);
        },
        Err(err) => out.push(ValidationError::new(path_id, "frontmatter", err.to_string())),
    }
}

fn page_id_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("<unknown>")
        .to_owned()
}

#[expect(
    clippy::print_stdout,
    reason = "CLI output by design — `stoa schema` prints STOA.md verbatim."
)]
fn print(text: &str) {
    print!("{text}");
}
