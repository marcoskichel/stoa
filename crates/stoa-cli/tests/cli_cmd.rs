//! trycmd-driven golden-file snapshot tests for the top-level `stoa` CLI surface.
//!
//! Scenarios live under `tests/cmd/*.trycmd`. Regenerate with:
//! `just e2e-review` (== `TRYCMD=overwrite cargo test -p stoa-cli --test cli_cmd`).

#[test]
fn cli_cmd_golden_files() {
    trycmd::TestCases::new().case("tests/cmd/*.trycmd");
}
