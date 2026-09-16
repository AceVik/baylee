//! Where the preview panel opens.
//!
//! A hand card has a place in the HUD's own layout and the bubble has always
//! pointed at it. Nothing else does: a permanent is on the felt, a pile is
//! beside a mat, a stack card is in a panel that scrolls — so those anchor at
//! the pointer, and the arithmetic that keeps the panel beside the pointer
//! rather than on top of it, off the window's own top edge and off the hand
//! bar, is the whole of the placement.
//! Reported twice, as two bugs: "the hand flickers" and "cards jump on
//! hover". They are one branch — the hand scrolling a hovered card into view
//! for a pointer that was already on it.

use super::*;
use crate::hud::hand::hand_scroll_to;

/// Thirty cards in a bar that fits about four: everything below is far
/// enough off the end to move if anything is going to.
fn crowded() -> (crate::hud::HandLayout, f32) {
    let available = 400.0;
    (hand_layout(30, HAND_CARD_W, available), available)
}

#[test]
fn a_pointer_hover_never_moves_the_hand() {
    let (layout, available) = crowded();
    for index in [0, 7, 29] {
        let after = hand_scroll_to(300.0, Some(index), true, layout, available);
        assert!(
            (after - 300.0).abs() < 1e-6,
            "card {index} is under the pointer, so it is already on the \
             screen — scrolling it into view is what made the hand jump, \
             and it moved to {after}"
        );
    }
}

#[test]
fn the_keyboard_cursor_pulls_its_card_into_view_from_either_end() {
    let (layout, available) = crowded();
    let far = hand_scroll_to(0.0, Some(20), false, layout, available);
    assert!(
        far > 0.0,
        "a card off the right-hand end is scrolled to, or the cursor is \
         on something nobody can see"
    );
    let start = 20.0 * layout.step;
    assert!(
        (far - (start + HAND_CARD_W - available)).abs() < 1e-3,
        "and only just far enough: its right edge lands on the bar's"
    );
    let back = hand_scroll_to(600.0, Some(2), false, layout, available);
    assert!(
        (back - 2.0 * layout.step).abs() < 1e-3,
        "and from the other side, its left edge on the bar's"
    );
}

#[test]
fn a_card_already_in_view_holds_the_hand_still() {
    let (layout, available) = crowded();
    let scroll = 5.0 * layout.step;
    assert!(
        (hand_scroll_to(scroll, Some(6), false, layout, available) - scroll).abs() < 1e-6,
        "the keyboard cursor moves the hand only when it has to"
    );
    assert!(
        (hand_scroll_to(scroll, None, false, layout, available) - scroll).abs() < 1e-6,
        "and a hover on nothing in the hand moves nothing at all"
    );
}
