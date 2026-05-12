//! Fixture: only doc comments. Must NOT be flagged.

/// Outer doc on a public function. Describes how the function behaves.
pub fn x() -> u32 {
    42
}

/** Outer block doc on a struct. Documents an invariant. */
pub struct Y;

/*! Inner block doc placeholder. */
