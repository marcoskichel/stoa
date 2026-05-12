#!/usr/bin/env bash
# Cold-cache benchmark: copy the binary to a fresh path each invocation,
# then time the first run. We can't drop the OS page cache without root,
# but copying the file ensures the inode + binary pages aren't already
# resident from the prior iteration. Approximates true cold start.

set -euo pipefail

cd "$(dirname "$0")"

N="${N:-200}"
SRC=./target/release/stoa-hook-spike
SCRATCH=/tmp/stoa-spike-cold

rm -rf "$SCRATCH"
mkdir -p "$SCRATCH"

RESULTS=/tmp/stoa-spike-cold-results.txt
: > "$RESULTS"

for i in $(seq 1 "$N"); do
    BIN="$SCRATCH/stoa-hook-spike-$i"
    DB="$SCRATCH/queue-$i.db"
    cp "$SRC" "$BIN"
    sync
    # posix_fadvise DONTNEED via dd would help but needs root or specific fs;
    # the cp+sync at minimum forces a fresh inode + cached blocks separate
    # from the prior iteration's pages.

    t0=$(date +%s%N)
    "$BIN" "$DB" "session-$i" >/dev/null
    t1=$(date +%s%N)
    echo $(( (t1 - t0) / 1000 )) >> "$RESULTS"
done

python3 - <<'PY'
import statistics
xs = sorted(int(l) for l in open("/tmp/stoa-spike-cold-results.txt") if l.strip())
n = len(xs)
def pct(p):
    k = int(round((p/100.0) * (n-1)))
    return xs[k]
print(f"n         {n}")
print(f"min       {xs[0]} us")
print(f"p50       {pct(50)} us")
print(f"p95       {pct(95)} us")
print(f"p99       {pct(99)} us")
print(f"max       {xs[-1]} us")
print(f"mean      {int(statistics.mean(xs))} us")
print(f"stdev     {int(statistics.pstdev(xs))} us")
PY
