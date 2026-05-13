//! Shared tokio runtime helper for the daemon-calling subcommands.

use std::future::Future;

use anyhow::{Context, Result};

/// Build a current-thread tokio runtime and drive `fut` to completion.
///
/// Centralized so every subcommand uses the same lightweight runtime
/// rather than each spinning up its own multi-thread variant.
pub(crate) fn block_on<F>(fut: F) -> Result<()>
where
    F: Future<Output = Result<()>>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;
    rt.block_on(fut)
}
