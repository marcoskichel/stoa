//! Fixture: bare `//` line comment with no allowed prefix. Must be flagged.

pub fn x() -> u32 {
    // this should be flagged
    42
}
