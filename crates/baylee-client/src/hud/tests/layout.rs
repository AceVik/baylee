use super::*;

#[test]
fn few_cards_spread_evenly_and_fully_visible() {
    let layout = hand_layout(5, 100.0, 1000.0);
    assert!(!layout.scrollable);
    assert!(layout.step >= 100.0, "cards never overlap when they fit");
    assert!(layout.content_width <= 1000.0);
}

#[test]
fn many_cards_overlap_but_keep_the_minimum_visible() {
    let layout = hand_layout(12, 100.0, 600.0);
    assert!(layout.step >= 30.0, "at least 30% of every card shows");
    assert!(layout.step < 100.0, "they must overlap to fit");
}

#[test]
fn beyond_the_minimum_overlap_the_bar_becomes_scrollable() {
    let layout = hand_layout(30, 100.0, 400.0);
    assert!(layout.scrollable);
    assert!((layout.step - 30.0).abs() < 1e-4, "clamped to the 30% rule");
    assert!(layout.content_width > 400.0);
}

#[test]
fn an_empty_hand_is_not_scrollable() {
    let layout = hand_layout(0, 100.0, 400.0);
    assert!(!layout.scrollable);
    assert!(layout.content_width.abs() < 1e-4);
}

/// A hand stands in the middle of the bar.
///
/// The spread is capped at a card and a little air — a two-card hand
/// stretched across a monitor is two cards a player has to look for — so
/// there is nearly always room left over, and it used to end up entirely
/// on the right while the cards sat against the left edge.
#[test]
fn a_hand_that_does_not_fill_the_bar_stands_in_the_middle_of_it() {
    for count in 1..=8 {
        let layout = hand_layout(count, 100.0, 1400.0);
        let left = layout.lead;
        let right = 1400.0 - (layout.lead + layout.content_width);
        assert!(
            (left - right).abs() < 1e-3,
            "{count} cards left {left} on one side and {right} on the other"
        );
    }
}

/// And a hand that overflows starts at the very edge, because the scroll
/// offset is measured from there: a lead that moved as cards were played
/// would drag the whole strip sideways under the player's pointer.
#[test]
fn a_hand_wider_than_the_bar_is_not_centred() {
    let layout = hand_layout(30, 100.0, 400.0);
    assert!(layout.scrollable);
    assert!(layout.lead.abs() < 1e-4, "it started {} in", layout.lead);
}

/// The hand gets the whole bar, and both readers of that number agree.
///
/// The rebuild took 110 pixels off the right-hand end for a commander
/// zone and the per-frame scroll system did not, so a seat with a
/// commander had its row spawned centred in one width and re-centred in
/// a wider one on the very next frame — half the zone, sideways and
/// back, on every rebuild. The hover is part of `HudRevision`, so a
/// pointer crossing the hand rebuilt it continuously and the row shook.
///
/// Neither should have subtracted it: the commander zone is drawn on the
/// table beside the mat and has been for some time, so what the
/// reservation held open was a hole.
#[test]
fn the_hand_is_laid_out_in_the_whole_bar() {
    let window = 1920.0;
    assert!(
        (hand_available(window) - (window - 2.0 * HAND_BAR_PAD)).abs() < 1e-3,
        "the hand gets everything but the bar's own padding, and it \
         answered {}",
        hand_available(window)
    );
}

/// The zone clips its children, so the room above a card is a promise
/// about the tallest thing that can stand out of one.
///
/// A glow cut off flat along a horizontal line is not a subtle fault — it
/// stops reading as a light and starts reading as a box drawn round the
/// card — and the promise is made by four numbers that can each move on
/// their own.
///
/// The room is the ledge *plus* the headroom, and that is the whole of
/// what the rebuild changed: [`hand::HAND_HEADROOM`] came down from 25 to
/// 12 because a halo reaching past it now runs on under an opaque shelf
/// instead of into the clip. Asserted here against the zone as a whole,
/// so the day somebody makes the shelf shallower to buy back a pixel of
/// table, this is what says no.
#[test]
fn the_zone_keeps_room_for_a_raised_card_and_its_glow() {
    use super::super::hand::{ARMED_RAISE, HALO_REACH, HAND_FOOTROOM};
    let room = HAND_ZONE_H - HAND_CARD_H - HAND_FOOTROOM;
    assert!(
        room >= ARMED_RAISE + HALO_REACH,
        "a card raised by {ARMED_RAISE} with a halo reaching {HALO_REACH} \
         has {room} to stand in"
    );
}

/// The zone is the camera's share of the window, and it did not grow.
///
/// `Canvas::hud.bottom` is this number and `CameraRig::home` frames the
/// table in what is left, so every pixel here is a pixel the table does
/// not get. The rebuild put a 40-pixel shelf along the top of the hand
/// and paid for it by narrowing the card from 110 to 92 — the ledger the
/// owner was promised, written where it can fail.
#[test]
fn the_ledge_is_paid_for_out_of_the_cards_and_not_out_of_the_table() {
    use super::super::hand::{HAND_FOOTROOM, HAND_HEADROOM, LEDGE_H};
    // What the bar it replaces came to: a 110-wide card, 25 of headroom,
    // 10 of footroom.
    let was = 110.0 * 88.0 / 63.0 + 25.0 + HAND_FOOTROOM;
    assert!(
        HAND_ZONE_H - was < 2.0,
        "the zone is {HAND_ZONE_H} against the bar's {was}, and the \
         difference is table"
    );
    assert!(
        (HAND_ZONE_H - (LEDGE_H + HAND_HEADROOM + HAND_CARD_H + HAND_FOOTROOM)).abs() < 1e-3,
        "the zone is its four parts and nothing else"
    );
}

/// A glow is not a shadow in another colour.
///
/// This is exactly what went wrong. The playable glow was one
/// `BoxShadow`, blur six, **spread zero** — and `soft_shadow`, which
/// every un-lit card in the bar wears, is blur six. Identical geometry;
/// and once the bar stopped painting an 88%-black strip behind them, very
/// nearly identical pictures, which is why the owner reported the glow as
/// gone. A light has to begin outside the card's edge, where the shadow
/// has already finished, and carry further than it does.
#[test]
fn a_glow_stands_off_the_card_further_than_its_own_shadow() {
    let Val::Px(plain) = soft_shadow()[0].blur_radius else {
        panic!("the drop shadow is measured in pixels");
    };
    let lit = super::super::hand::halo(palette::ACTIVE, 1.0);
    let (Val::Px(spread), Val::Px(blur)) = (lit[0].spread_radius, lit[0].blur_radius) else {
        panic!("so is the halo");
    };
    assert!(
        spread > 0.0,
        "a glow with no spread starts falling off at the card's own edge, \
         which is where the shadow starts too"
    );
    assert!(
        blur > plain,
        "and it has to carry further than the shadow: {blur} against {plain}"
    );
}

/// And the preview points at the card, not at where the row would have
/// started if it were not centred.
///
/// `lead` is half the zone's spare room, so the error this catches grew
/// as the hand shrank — the fewer cards left, the further from its card
/// the bubble stood.
///
/// Checked against **symmetry** and not against the sum. The test that
/// was here added the same terms in the same order as the function and
/// so agreed with it however wrong both were, which is exactly what
/// happened: a `HAND_BAR_PAD` that the layout does not apply sat in both
/// for as long as either existed, and it took a screenshot to see it. A
/// centred row's middle card is on the window's own centre line and its
/// ends are equidistant from it — neither of which survives a constant
/// offset, and neither of which is this function written out twice.
#[test]
fn the_preview_stands_on_the_card_it_describes() {
    const WINDOW: f32 = 1920.0;
    let layout = hand_layout(7, HAND_CARD_W, hand_available(WINDOW));
    assert!(layout.lead > 100.0, "this window has room to centre in");
    let at = |i| crate::hud::hand::hand_card_x(layout, 0.0, i);
    assert!(
        (at(3) - WINDOW / 2.0).abs() < 1e-3,
        "the middle of seven cards is at {} and the window's middle is {}",
        at(3),
        WINDOW / 2.0
    );
    assert!(
        ((at(3) - at(0)) - (at(6) - at(3))).abs() < 1e-3,
        "the row reaches {} to the left of centre and {} to the right",
        at(3) - at(0),
        at(6) - at(3)
    );
}
