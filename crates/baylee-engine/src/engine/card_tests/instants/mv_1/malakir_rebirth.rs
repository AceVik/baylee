//! `cards/instants/mv_1/malakir_rebirth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Malakir Rebirth, played as its **back** face: a land that enters tapped
/// and makes black.
///
/// The front face's granted death trigger is refused by name, so the back is
/// where this printing is testable — and the two halves of what a modal
/// double-faced land carries are exactly `EnterModifier::Tapped` and the mana
/// ability behind it.
#[test]
fn malakir_caverns_enters_tapped_and_makes_black() {
    let (engine, land) = play_land_face(malakir_rebirth(), 1).expect("the back face is a land");
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\" is an enter modifier and not a line the \
         harness can place around"
    );
}
