//! `cards/sorceries/mv_1/prismatic_ending.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The same {2}{W} paid with three Plains is one color. The target was
/// legal — the condition is not part of the targeting — so the spell
/// resolves and the three-drop stays.
#[test]
fn prismatic_ending_of_one_color_leaves_a_three_drop() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let engine = prismatic_ending_at(&[plains(), plains(), plains()], 2, prismatic_three_drop());
    assert!(on_battlefield(&engine, p1, prismatic_three_drop()).is_some());
    assert!(in_graveyard(&engine, p0, prismatic_ending()).is_some());
}
