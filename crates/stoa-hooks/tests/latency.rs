//! E2E quality gate: `stoa-hook` cold-start latency.
//!
//! Spec source: [ROADMAP.md M3 — exit criteria].
//!
//! ROADMAP demands **<10ms p95** on Linux + macOS. Per the M3 research note
//! on benchmark methodology, GitHub Actions shared runners introduce 2-5ms
//! of scheduler jitter, so this test enforces a relaxed **<15ms p95**
//! threshold; the strict 10ms gate is enforced separately via hyperfine in
//! a dedicated CI job (see `scripts/bench-hook-latency.sh`).
//!
//! Opt-in: set `STOA_LATENCY_GATE=1` to run. Default-off so `cargo test`
//! stays fast.

mod common;

use std::time::{Duration, Instant};

use common::{fresh_queue_path, run_hook};

const SAMPLES: usize = 50;
const P95_THRESHOLD_MS: u128 = 15;

fn latency_gate_enabled() -> bool {
    std::env::var("STOA_LATENCY_GATE").is_ok_and(|v| v == "1")
}

fn measure_one(queue: &std::path::Path, n: usize) -> Duration {
    let start = Instant::now();
    let _ignored = run_hook(queue, &format!("sess-{n}"), "/tmp/raw.jsonl");
    start.elapsed()
}

#[test]
fn hook_cold_start_p95_under_15ms() {
    if !latency_gate_enabled() {
        return;
    }
    let (_tmp, queue) = fresh_queue_path();
    let mut samples: Vec<Duration> = (0..SAMPLES).map(|n| measure_one(&queue, n)).collect();
    samples.sort();
    let p95_index = SAMPLES.saturating_mul(95).div_ceil(100).saturating_sub(1);
    let p95 = samples[p95_index];
    assert!(
        p95.as_millis() < P95_THRESHOLD_MS,
        "stoa-hook p95 cold-start {p95:?} exceeds {P95_THRESHOLD_MS}ms gate (samples: {SAMPLES})",
    );
}
