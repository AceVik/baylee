//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn wolverine_pack_counts_current_blockers_on_resolution() {
    assert_printed_rampage(index::WOLVERINE_PACK, 2);
}
