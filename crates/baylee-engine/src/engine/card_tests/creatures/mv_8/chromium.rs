//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn chromium_counts_current_blockers_on_resolution() {
    assert_printed_rampage(index::CHROMIUM, 2);
}
