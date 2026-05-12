//! `stoa read <id>` — print a wiki page (frontmatter + body) to stdout.
//!
//! Routing follows the id prefix (see `stoa_core::Id`):
//!   `ent-*` → `wiki/entities/`, `con-*` → `wiki/concepts/`,
//!   `syn-*` → `wiki/synthesis/`.

use anyhow::{Context, anyhow};

use crate::workspace::Workspace;

/// Run `stoa read <id>` from the current working directory.
pub(crate) fn run(id: &str) -> anyhow::Result<()> {
    let ws = Workspace::current()?;
    let path = ws.page_path(id)?;
    if !path.is_file() {
        return Err(anyhow!("page `{id}` not found at {}", path.display()));
    }
    let body = std::fs::read_to_string(&path)
        .with_context(|| format!("reading page `{}`", path.display()))?;
    print(&body);
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "CLI output by design — `stoa read` writes the page to stdout."
)]
fn print(body: &str) {
    print!("{body}");
}
