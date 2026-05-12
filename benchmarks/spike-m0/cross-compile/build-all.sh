#!/usr/bin/env bash
# Cross-compile the hook spike binary to all 5 v0.1 release targets.
# Records: target / status / binary size / build time / notes.
# Note: macOS targets need the Apple SDK (osxcross). cross-rs does not
# ship a macOS image; document failure mode.

set -uo pipefail

cd "$(dirname "$0")"

# Prepend rustup-managed cargo so cross-rs can detect the active toolchain.
# Without this, /usr/bin/cargo (Arch system package) wins and cross misparses
# the toolchain name as "usr".
export PATH="$HOME/.cargo/bin:$PATH"
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}"

# Wine inside the cross-rs windows-gnu image fails on Linux >=6.x with the
# default Docker seccomp profile ("socket: Function not implemented"). Disable
# seccomp for the build container only. CI alternatives: GitHub Actions
# windows-latest runner does native Windows builds without Wine.
export CROSS_CONTAINER_OPTS="${CROSS_CONTAINER_OPTS:---security-opt seccomp=unconfined}"

TARGETS=(
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    "x86_64-pc-windows-gnu"
)

REPORT=build-report.tsv
echo -e "target\tstatus\tsize_bytes\tduration_sec\tnotes" > "$REPORT"

for T in "${TARGETS[@]}"; do
    echo "=== $T ==="
    t0=$SECONDS
    LOG="build-${T}.log"

    case "$T" in
        x86_64-unknown-linux-gnu)
            # Native build, no cross needed
            cargo build --release --target "$T" >"$LOG" 2>&1
            STATUS=$?
            NOTES="native"
            ;;
        x86_64-pc-windows-gnu)
            # cross has a working image for windows-gnu
            cross build --release --target "$T" >"$LOG" 2>&1
            STATUS=$?
            NOTES="cross+mingw"
            ;;
        aarch64-unknown-linux-gnu)
            cross build --release --target "$T" >"$LOG" 2>&1
            STATUS=$?
            NOTES="cross+linux-aarch64"
            ;;
        x86_64-apple-darwin|aarch64-apple-darwin)
            cross build --release --target "$T" >"$LOG" 2>&1
            STATUS=$?
            NOTES="needs-osxcross-image"
            ;;
    esac

    dur=$((SECONDS - t0))

    if [ $STATUS -eq 0 ]; then
        BIN="target/$T/release/stoa-hook-spike"
        [ "$T" = "x86_64-pc-windows-gnu" ] && BIN="${BIN}.exe"
        if [ -f "$BIN" ]; then
            SIZE=$(stat -c '%s' "$BIN")
            echo -e "${T}\tOK\t${SIZE}\t${dur}\t${NOTES}" >> "$REPORT"
        else
            echo -e "${T}\tNO_BINARY\t0\t${dur}\t${NOTES}" >> "$REPORT"
        fi
    else
        echo -e "${T}\tFAIL\t0\t${dur}\t${NOTES}" >> "$REPORT"
    fi
done

echo
echo "=== summary ==="
cat "$REPORT" | column -t -s $'\t'
