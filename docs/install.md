# Install

Stoa needs two things on disk: the Rust binaries (`stoa`, `stoa-hook`, `stoa-inject-hook`) and the Python daemon (`stoa-recalld`, plus optionally `stoa-harvest` and `stoa-crystallize`).

## Requirements

- **Rust 1.95+** via [`rustup`](https://rustup.rs).
- **Python 3.13+** and [`uv`](https://docs.astral.sh/uv/) for the daemon.
- **[MemPalace](https://github.com/MemPalace/mempalace)** 3.3.5+ — the retrieval substrate.

## Quick install

```bash
# 1. Install MemPalace (the retrieval substrate)
uv tool install mempalace

# 2. Install Stoa's Rust binaries
cargo install stoa-cli stoa-hooks stoa-inject-hooks --locked

# 3. Install Stoa's Python sidecar
uv tool install stoa-recalld

# 4. (Optional) Install the LLM workers
uv tool install stoa-harvest stoa-crystallize    # needs ANTHROPIC_API_KEY
```

## What ends up where

| Binary | Source | Where it lives |
|---|---|---|
| `stoa` | `stoa-cli` | `~/.cargo/bin/stoa` |
| `stoa-hook` | `stoa-hooks` | `~/.cargo/bin/stoa-hook` |
| `stoa-inject-hook` | `stoa-inject-hooks` | `~/.cargo/bin/stoa-inject-hook` |
| `stoa-recalld` | Python daemon | `~/.local/bin/stoa-recalld` (via uv) |
| `mempalace` | MemPalace CLI | `~/.local/bin/mempalace` |
| `stoa-harvest` / `stoa-crystallize` | LLM workers | `~/.local/bin/...` |

## From source

```bash
git clone https://github.com/marcoskichel/stoa
cd stoa
just install-dev
```

`just install-dev` runs `cargo install --path crates/stoa-cli` (and the other Rust crates) + `uv sync --all-groups` for the Python workspace.

## Verify

```bash
stoa --version
mempalace --version
stoa-recalld --help    # ensure entry point is on PATH
```

If `stoa-recalld --help` fails with "command not found", make sure `~/.local/bin` is on your `PATH` (`uv tool install` puts entry points there by default).

## Configuration

Stoa is zero-config out of the box. You can override defaults via environment variables:

| Var | Default | Purpose |
|---|---|---|
| `STOA_RECALLD_SOCKET` | `$XDG_RUNTIME_DIR/stoa-recalld.sock` | Daemon socket path |
| `STOA_PALACE_PATH` | `<workspace>/.stoa/palace` | Where MemPalace stores its ChromaDB segment |
| `STOA_RECALLD_BIN` | `stoa-recalld` | Binary `stoa daemon start` should spawn |
| `STOA_RECALLD_PID_FILE` | `$XDG_RUNTIME_DIR/stoa-recalld.pid` | PID file for `stoa daemon stop` |
| `STOA_RECALLD_LOG_FILE` | `$XDG_STATE_HOME/stoa/recalld.log` | Where the daemon's stdout/stderr lands |

The Rust client + Python daemon both resolve `STOA_RECALLD_SOCKET` identically so cross-binary handoffs stay consistent.

## Next

- [Quickstart](quickstart.md) — 5 commands to a working workspace.
- [Troubleshooting](troubleshooting.md) — common install failures.
