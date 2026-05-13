//! Scoring logic for the `AgentLeak` adapter.
//!
//! For each case the redactor's output is checked against the
//! `private_vault` canary tokens. A case is correct iff **no** canary
//! string survives verbatim in the output (when `expected_redacted` is
//! true), or the input is preserved unchanged (for benign controls).
//! Smoke cases ship with no canary tokens and fall back to a
//! difference-from-input proxy.

use std::collections::BTreeMap;

use stoa_capture::Redactor;

use super::Case;

pub(crate) fn score_cases(cases: &[Case]) -> BTreeMap<String, f64> {
    let redactor = Redactor::with_defaults();
    let mut tally = Tally::default();
    for case in cases {
        let redacted = redactor.redact_line(&case.input);
        let (correct, missed) = score_case(case, &redacted);
        tally.record(&case.channel, case.family.as_deref(), correct, missed);
    }
    finalize(&tally, cases.len())
}

/// Score one case against its redacted output.
///
/// When the case carries vault canary tokens (full corpus path), the
/// "leaked" predicate is "any canary token survives verbatim in the
/// output". Smoke cases ship with no tokens and fall back to a
/// difference-from-input proxy.
///
/// Returns `(correct, missed_leak)`:
/// - `correct` — case scored success.
/// - `missed_leak` — case expected redaction and at least one canary
///   leaked through.
fn score_case(case: &Case, redacted: &str) -> (bool, bool) {
    if case.canary_tokens.is_empty() {
        return score_by_difference(case, redacted);
    }
    let leaked = case
        .canary_tokens
        .iter()
        .any(|t| redacted.contains(t.as_str()));
    if case.expected_redacted {
        (!leaked, leaked)
    } else {
        (!leaked, false)
    }
}

fn score_by_difference(case: &Case, redacted: &str) -> (bool, bool) {
    let was_redacted = redacted != case.input;
    let correct = was_redacted == case.expected_redacted;
    let missed = case.expected_redacted && !was_redacted;
    (correct, missed)
}

#[derive(Default)]
struct Bucket {
    total: usize,
    correct: usize,
    missed_leaks: usize,
}

impl Bucket {
    fn observe(&mut self, correct: bool, missed: bool) {
        self.total += 1;
        if correct {
            self.correct += 1;
        }
        if missed {
            self.missed_leaks += 1;
        }
    }
}

#[derive(Default)]
struct Tally {
    by_channel: BTreeMap<String, Bucket>,
    by_family: BTreeMap<String, Bucket>,
}

impl Tally {
    fn record(&mut self, channel: &str, family: Option<&str>, correct: bool, missed: bool) {
        self.by_channel
            .entry(channel.to_owned())
            .or_default()
            .observe(correct, missed);
        let fam_key = family.unwrap_or("F0").to_owned();
        self.by_family
            .entry(fam_key)
            .or_default()
            .observe(correct, missed);
    }
}

fn finalize(tally: &Tally, total_cases: usize) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    let (overall_correct, overall_missed) =
        aggregate(&tally.by_channel, "accuracy", "missed_leaks", &mut out);
    aggregate(&tally.by_family, "accuracy", "missed_leaks", &mut out);
    out.insert("accuracy".to_owned(), safe_div(overall_correct, total_cases));
    out.insert("missed_leaks".to_owned(), safe_div(overall_missed, total_cases));
    out
}

fn aggregate(
    buckets: &BTreeMap<String, Bucket>,
    acc_prefix: &str,
    missed_prefix: &str,
    out: &mut BTreeMap<String, f64>,
) -> (usize, usize) {
    let mut correct = 0usize;
    let mut missed = 0usize;
    for (label, b) in buckets {
        out.insert(format!("{acc_prefix}:{label}"), safe_div(b.correct, b.total));
        out.insert(format!("{missed_prefix}:{label}"), safe_div(b.missed_leaks, b.total));
        correct += b.correct;
        missed += b.missed_leaks;
    }
    (correct, missed)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "case counts bounded by corpus size; well under 2^52"
)]
fn safe_div(num: usize, denom: usize) -> f64 {
    if denom == 0 {
        0.0
    } else {
        num as f64 / denom as f64
    }
}
