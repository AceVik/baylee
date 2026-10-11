//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn sea_kings_blessing_changes_every_legal_target_until_cleanup() {
    assert_plural_color_change(index::SEA_KINGS_BLESSING, Color::Blue, ManaColor::Blue);
}
