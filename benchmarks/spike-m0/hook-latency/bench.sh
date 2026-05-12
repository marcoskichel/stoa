#!/usr/bin/env bash
# Cold-start hook latency benchmark.
# Runs the spike binary N times, recording wall-clock time per invocation.
# Drops the page cache before each run is impractical without sudo, so we
# warm the cache once then measure (this matches reality - hot-path daemon
# was already running when an editor session ends).
#
# Output: prints p50 / p95 / p99 / max in microseconds, and a histogram.

set -euo pipefail

cd "$(dirname "$0")"

N="${N:-1000}"
DB=/tmp/stoa-spike-queue.db
BIN=./target/release/stoa-hook-spike

if [ ! -x "$BIN" ]; then
    echo "build first: cargo build --release" >&2
    exit 1
fi

rm -f "$DB" "$DB-shm" "$DB-wal"

# Warm the binary into the OS page cache and create the schema.
"$BIN" "$DB" warmup >/dev/null

# Measure with a tight loop, recording nanoseconds per invocation.
RESULTS=/tmp/stoa-spike-results.txt
: > "$RESULTS"

for i in $(seq 1 "$N"); do
    # nanosecond-resolution wall clock around the invocation.
    t0=$(date +%s%N)
    "$BIN" "$DB" "session-$i" >/dev/null
    t1=$(date +%s%N)
    echo $(( (t1 - t0) / 1000 )) >> "$RESULTS"  # microseconds
done

# Stats
python3 - <<'PY'
import statistics
xs = sorted(int(l) for l in open("/tmp/stoa-spike-results.txt") if l.strip())
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
