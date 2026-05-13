# Troubleshooting

Common first-run failures and how to recover.

## "`stoa: command not found`"

The CLI is installed via `cargo install` and lives at `~/.cargo/bin/stoa`.
That directory has to be on your `PATH`:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc   # or ~/.zshrc
exec $SHELL
```

If you used `--git` install: confirm `cargo install --git ...` exited
without errors. Compile errors there get swallowed if you skip the
last lines of output.

## `cross: invalid toolchain name: 'usr'` (Arch Linux)

Arch ships Rust as a system package at `/usr/bin/cargo`. That shadows
the rustup-managed cargo at `~/.cargo/bin/cargo`, and `cross-rs` reads
rustup metadata that does not exist for the system Rust:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export RUSTUP_TOOLCHAIN=stable
```

This is documented in
[CONTRIBUTING.md](https://github.com/marcoskichel/stoa/blob/main/CONTRIBUTING.md)
under "Environment traps".

## `wine: socket : Function not implemented` (cross-build to Windows on Linux ≥ 7.x)

The seccomp profile inside the cross-rs Windows-gnu container blocks a
syscall wine needs:

```bash
export CROSS_CONTAINER_OPTS="--security-opt seccomp=unconfined"
```

Local-only; production CI builds Windows on a native runner.

## "Hook fired but nothing landed in `sessions/`"

The capture pipeline is asynchronous. The hook only enqueues a row;
the actual write happens in the daemon's capture worker. Check the
daemon is running:

```bash
pgrep -fa 'stoa daemon'
```

If it is not, start it:

```bash
stoa daemon &
```

Then check the audit log:

```bash
tail -n 5 .stoa/audit.log
```

You should see one `stoa.capture` event per processed row.

## `stoa query` returns no hits

Three causes are common:

1. **Index never built.** Run `stoa rebuild`. This regenerates
   `.stoa/recall.db` and `.stoa/vectors/` from `wiki/`, `raw/`, and
   `sessions/`.
2. **Embedding model not downloaded.** On first run Stoa fetches
   `bge-small-en-v1.5`. Watch for network errors in daemon stderr. If
   the model is unavailable, fall back to BM25-only by re-running
   `stoa init --no-embeddings` (warning: this resets the index).
3. **Query is below the relevance floor.** SessionStart injection skips
   anything below a configured floor to avoid noise. `stoa query`
   itself does not apply the floor; if `query` returns 0 hits, the
   issue is index coverage, not gating.

## SessionStart injection is empty

Run:

```bash
stoa inject log --limit 1
```

The audit entry shows `hits=N`. If `hits=0`, Stoa did not find anything
relevant for the session's cwd / git remote / recent wiki pages. This
is the intended behavior — Stoa does not inject when it has no signal.

If `hits>0` but you do not see the `<stoa-memory>` block in the agent's
context: confirm the hook is actually registered:

```bash
stoa hook install --platform claude-code --inject session-start
```

(`install` is idempotent — safe to re-run.)

## "Injected snippet contains `<stoa-memory>` inside a snippet body"

Stoa's MINJA defense splices a U+2060 word joiner inside any
`<stoa-memory` or `</stoa-memory` substring in snippet bodies, source
paths, and queries. The wrapped text renders identically to humans;
the joiner stops the snippet from closing the envelope.

If you see what looks like an unescaped tag inside the block, copy it
into a hex viewer — the word joiner is invisible but present. The
regression test asserts that exactly **one** open tag and **one** close
tag survive sanitization regardless of snippet content.

## Hook latency feels slow

The capture hook target is **<10 ms p95**. If the agent UI feels
sluggish on session end, the daemon may be holding the queue lock or
the hook binary may not be the one you installed. Check:

```bash
which stoa-hook
stoa-hook --version
```

The path must point at a release-mode binary, not a debug build.
`cargo install` defaults to release; `just install-dev` also does.

## Where else to look

- `ARCHITECTURE.md` — load-bearing invariants and design rationale.
- `ROADMAP.md` — what is shipped vs what is deferred.
- [GitHub Issues](https://github.com/marcoskichel/stoa/issues) — open
  one if your case is not covered above. The issue template prompts
  for the exact reproduction info maintainers need.
