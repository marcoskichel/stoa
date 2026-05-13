//! `stoa schema` — print or validate STOA.md.
//!
//! v0.1 ships the print path + a `--check` path that walks `wiki/*.md`
//! and runs each page through `stoa_core::validate_page`. The check
//! lives in the CLI so it does not require the daemon to be up.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use stoa_core::{Schema, validate_page};

use crate::cli::SchemaArgs;

/// Run `stoa schema`.
pub(crate) fn run(args: &SchemaArgs) -> Result<()> {
    let root = resolve_workspace_root()?;
    let stoa_md = root.join("STOA.md");
    let raw =
        fs::read_to_string(&stoa_md).with_context(|| format!("reading {}", stoa_md.display()))?;
    if args.check {
        let schema = Schema::from_stoa_md(&raw);
        check_pages(&root, &schema)?;
        println("OK");
        return Ok(());
    }
    println(&raw);
    Ok(())
}

fn check_pages(root: &Path, schema: &Schema) -> Result<()> {
    let mut error_lines: Vec<String> = Vec::new();
    walk_md(&root.join("wiki"), &mut |path| {
        for line in validate_one(path, schema) {
            error_lines.push(line);
        }
    });
    if !error_lines.is_empty() {
        for e in &error_lines {
            println(e);
        }
        bail!("{} page(s) failed validation", error_lines.len());
    }
    Ok(())
}

fn validate_one(path: &Path, schema: &Schema) -> Vec<String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return vec![format!("{}: unable to read file", path.display())];
    };
    let Some((fm_yaml, _body)) = split_frontmatter(&raw) else {
        return vec![format!(
            "{}: missing or malformed YAML frontmatter",
            path.display()
        )];
    };
    let path_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("<unknown>")
        .to_owned();
    let errs = validate_page(fm_yaml, &path_id, schema);
    errs.into_iter()
        .map(|e| format!("{}: {e}", path.display()))
        .collect()
}

fn split_frontmatter(raw: &str) -> Option<(&str, &str)> {
    let rest = raw.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let fm = &rest[..end];
    let after = &rest[end + 4..];
    let body = after.strip_prefix('\n').unwrap_or(after);
    Some((fm, body))
}

fn walk_md(dir: &Path, on_file: &mut impl FnMut(&Path)) {
    let Ok(read) = fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_md(&path, on_file);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            on_file(&path);
        }
    }
}

fn resolve_workspace_root() -> Result<PathBuf> {
    let here = std::env::current_dir().context("getting current dir")?;
    let mut cursor: Option<&Path> = Some(&here);
    while let Some(d) = cursor {
        if d.join("STOA.md").is_file() {
            return Ok(d.to_path_buf());
        }
        cursor = d.parent();
    }
    bail!("no STOA.md found from `{}` up to /", here.display());
}

#[expect(clippy::print_stdout, reason = "User-facing CLI output.")]
fn println(msg: &str) {
    println!("{msg}");
}
