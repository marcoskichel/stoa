# Install

Stoa is a single Rust binary plus an optional Python sidecar. The
sidecar bootstraps automatically on the first `stoa daemon` run, so
day-to-day you only manage the Rust side.

## Supported platforms

| OS      | Architectures        |
| ------- | -------------------- |
| Linux   | `x86_64`, `aarch64`  |
| macOS   | `x86_64`, `aarch64`  |
| Windows | `x86_64`             |

Each release tag publishes tarballs for all five targets via the
[release workflow](https://github.com/marcoskichel/stoa/blob/main/.github/workflows/release.yml).

## Prerequisites

- **Rust toolchain** via [`rustup`](https://rustup.rs) (the workspace
  pins `1.95` in `rust-toolchain.toml`).
- **`uv`** ([astral.sh/uv](https://docs.astral.sh/uv/)) only if you run
  the Python sidecar locally for development. End users get it
  bootstrapped transparently.

## Pre-release (today)

The v0.1 tag has not landed yet. Install directly from `main`:

```bash
cargo install --git https://github.com/marcoskichel/stoa \
    stoa-cli stoa-hooks stoa-inject-hooks
```

This compiles three binaries into `~/.cargo/bin`:

- `stoa` — the CLI.
- `stoa-hook` — capture hook fired by `Stop` / `SessionEnd`.
- `stoa-inject-hook` — injection hook fired by `SessionStart`.

Add `~/.cargo/bin` to `PATH` if rustup did not do it already.

## Stable (after v0.1 ships)

```bash
cargo install stoa-cli stoa-hooks stoa-inject-hooks
```

## From a release tarball

Download a tarball from the
[Releases page](https://github.com/marcoskichel/stoa/releases) and
extract:

```bash
tar -xzf stoa-x86_64-unknown-linux-gnu.tar.gz
# Move the unpacked binaries somewhere on $PATH, e.g.:
sudo install -m 0755 stoa stoa-hook /usr/local/bin/
```

The tarball currently contains the `stoa` CLI and the `stoa-hook`
capture binary. The `stoa-inject-hook` SessionStart binary will ship in
the same tarball once the
[release pipeline tracks it](https://github.com/marcoskichel/stoa/blob/main/ROADMAP.md)
— scheduled for M6 alongside the v0.1 tag. Until then, install
`stoa-inject-hook` via the `cargo install` invocation above.

## Building from source

```bash
git clone https://github.com/marcoskichel/stoa
cd stoa
./scripts/bootstrap.sh        # installs dev tools + builds both workspaces
just install-dev              # cargo install stoa-cli + uv sync sidecar
```

`just install-dev` puts the CLI on your `PATH` from a local checkout.
Use this when you are iterating on Stoa itself.

## Verify

```bash
stoa --version
```

You should see `stoa <version>` printed (the exact version depends on
your install source — `main` shows a pre-release identifier; tagged
releases show `0.X.Y`).

## Next

- [Quickstart](quickstart.md) walks through the value loop in five
  commands.
