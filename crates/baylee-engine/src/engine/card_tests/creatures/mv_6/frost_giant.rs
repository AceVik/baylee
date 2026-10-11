//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn frost_giant_counts_current_blockers_on_resolution() {
    assert_printed_rampage(index::FROST_GIANT, 2);
}
