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
cargo install --git https://github.com/marcoskichel/stoa stoa-cli
```

This compiles and installs the `stoa` and `stoa-hook` binaries into
`~/.cargo/bin`. Add that directory to `PATH` if rustup did not do it
already.

## Stable (after v0.1 ships)

```bash
cargo install stoa-cli
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

The tarball contains the `stoa` CLI and the `stoa-hook` capture binary.
The `stoa-inject-hook` SessionStart binary ships in the same tarball
once the [release pipeline tracks it](https://github.com/marcoskichel/stoa/blob/main/ROADMAP.md)
(M6).

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

```console
$ stoa --version
stoa 0.1.0
```

## Next

- [Quickstart](quickstart.md) walks through the value loop in five
  commands.
