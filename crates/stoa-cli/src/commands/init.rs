//! `stoa init` — scaffold a Stoa workspace.
//!
//! Creates `STOA.md` (default schema), `wiki/{entities,concepts,synthesis}/`,
//! `raw/`, `sessions/`, and `.stoa/` (audit log + daemon socket dir if
//! the platform lacks `$XDG_RUNTIME_DIR`). Idempotent — re-running on
//! an existing workspace leaves STOA.md and existing pages untouched.

use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::InitArgs;

const DEFAULT_STOA_MD: &str = r#"# STOA.md — workspace schema

Stoa loads this file on every command. Add bullet items under the
"Entity types" and "Relationship types" headings to extend the
vocabulary the wiki will accept. Pages that violate the schema fail
`stoa schema --check`.

# Entity types

- `library` — third-party code you depend on
- `service` — running process or daemon
- `tool` — CLI or developer-facing utility
- `team` — group of people
- `concept` — domain idea or abstraction

# Relationship types

- `uses`
- `depends_on`
- `related_to`
- `supersedes`
"#;

/// Run `stoa init`.
pub(crate) fn run(args: InitArgs) -> Result<()> {
    let root = args.dir;
    create_dir_all(&root)?;
    write_if_missing(&root.join("STOA.md"), DEFAULT_STOA_MD)?;
    for sub in [
        "wiki/entities",
        "wiki/concepts",
        "wiki/synthesis",
        "raw",
        "sessions",
        ".stoa",
    ] {
        create_dir_all(&root.join(sub))?;
    }
    write_if_missing(&root.join(".gitignore"), ".stoa/\n")?;
    println(&format!("Scaffolded Stoa workspace at {}", root.display()));
    println(
        "Next: `stoa daemon start` to launch the recall daemon, then `stoa write` to add pages.",
    );
    Ok(())
}

fn create_dir_all(p: &Path) -> Result<()> {
    fs::create_dir_all(p).with_context(|| format!("creating {}", p.display()))
}

fn write_if_missing(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let mut f = fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    f.write_all(content.as_bytes())
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[expect(clippy::print_stdout, reason = "User-facing CLI status output.")]
fn println(msg: &str) {
    println!("{msg}");
}
