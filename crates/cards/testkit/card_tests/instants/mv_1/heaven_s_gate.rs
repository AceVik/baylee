//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn heaven_s_gate_changes_every_legal_target_until_cleanup() {
    assert_plural_color_change(index::HEAVEN_S_GATE, Color::White, ManaColor::White);
}
