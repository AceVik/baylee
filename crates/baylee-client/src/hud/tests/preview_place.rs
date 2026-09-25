use super::*;

/// A laptop's window, in logical pixels.
const WINDOW: Vec2 = Vec2::new(1728.0, 1052.0);
/// The preview at its default scale, padding included.
const PANEL: Vec2 = Vec2::new(320.0, 442.0);

fn covers(place: Vec2, panel: Vec2, point: Vec2) -> bool {
    point.x >= place.x
        && point.x <= place.x + panel.x
        && point.y >= place.y
        && point.y <= place.y + panel.y
}

/// The preview describes the card under the pointer, so a panel that
/// lands *on* the pointer is describing something the player can no
/// longer see — and, before `Pickable::IGNORE`, was also taking the
/// hover that opened it.
#[test]
fn the_panel_never_lands_under_the_pointer_that_opened_it() {
    for x in [12.0_f32, 200.0, 864.0, 1400.0, 1716.0] {
        for y in [60.0_f32, 300.0, 526.0, 870.0] {
            let at = Vec2::new(x, y);
            let place = preview_place(PreviewAt::Pointer(at), PANEL, WINDOW, None);
            assert!(
                !covers(place, PANEL, at),
                "a pointer at {at} opened a panel at {place} that covers it"
            );
        }
    }
}

/// Whatever it is anchored to, the panel stays inside the part of the
/// window a player can see it in — which is the whole of it now bar the
/// hand at the bottom.
#[test]
fn the_panel_stays_in_the_band() {
    let anchors = [
        PreviewAt::Loose,
        PreviewAt::Hand(20.0),
        PreviewAt::Hand(1700.0),
        PreviewAt::Pointer(Vec2::new(4.0, 8.0)),
        PreviewAt::Pointer(Vec2::new(1724.0, 1040.0)),
        PreviewAt::Pointer(Vec2::new(900.0, 60.0)),
    ];
    for at in anchors {
        let place = preview_place(at, PANEL, WINDOW, None);
        assert!(place.x >= 0.0, "{at:?} put the panel off the left: {place}");
        assert!(
            place.x + PANEL.x <= WINDOW.x,
            "{at:?} put the panel off the right: {place}"
        );
        assert!(place.y >= 0.0, "{at:?} put the panel off the top: {place}");
        assert!(
            place.y + PANEL.y <= WINDOW.y,
            "{at:?} put the panel off the bottom: {place}"
        );
    }
}

/// A pointer anchor is also kept off the window's own top edge and out
/// from under the hand zone, which the clamp above allows and this does
/// not: a preview whose bottom half is behind the hand is a preview of a
/// card's top half.
#[test]
fn a_pointer_anchor_clears_the_window_edge_and_the_hand_bar() {
    for y in [0.0_f32, 30.0, 500.0, 1000.0, 1052.0] {
        let place = preview_place(PreviewAt::Pointer(Vec2::new(600.0, y)), PANEL, WINDOW, None);
        assert!(place.y >= EDGE, "at y {y} the panel starts at {}", place.y);
        assert!(
            place.y + PANEL.y <= WINDOW.y - HAND_ZONE_H,
            "at y {y} the panel ends at {}",
            place.y + PANEL.y
        );
    }
}

/// The fault the card anchor exists for: a permanent on the felt is about
/// a hundred pixels across, and the pointer finds it at its *rim*, so a
/// panel opened a gap away from the pointer opened the better part of a
/// card's width inside the card and covered the thing it was describing.
///
/// Measured on a live table before the fix: an Island at logical
/// (895, 698) opened a panel whose left edge was on the card and whose
/// body crossed the middle of the board.
#[test]
fn a_card_anchor_opens_beside_the_card_and_never_over_it() {
    // A card as the camera draws one, walked across the table.
    let (w, h) = (104.0_f32, 146.0_f32);
    for cx in [90.0_f32, 400.0, 864.0, 1300.0, 1640.0] {
        for cy in [200.0_f32, 500.0, 800.0] {
            let rect = Rect {
                min: Vec2::new(cx - w / 2.0, cy - h / 2.0),
                max: Vec2::new(cx + w / 2.0, cy + h / 2.0),
            };
            let place = preview_place(PreviewAt::Card(rect), PANEL, WINDOW, None);
            let panel = Rect {
                min: place,
                max: place + PANEL,
            };
            // The two rectangles do not meet horizontally. Vertically
            // they always do — the panel is three times a card's height,
            // so it is bound to span it.
            assert!(
                panel.max.x <= rect.min.x || panel.min.x >= rect.max.x,
                "a card at {rect:?} opened a panel at {panel:?} over itself"
            );
        }
    }
}

/// The same anchor obeys the band. Written out because the card arm has
/// its own clamp and a test of the pointer arm would not have caught a
/// card one that was missing it.
#[test]
fn a_card_anchor_clears_the_window_edge_and_the_hand_bar() {
    for cy in [0.0_f32, 40.0, 600.0, 1052.0] {
        let rect = Rect {
            min: Vec2::new(800.0, cy - 73.0),
            max: Vec2::new(904.0, cy + 73.0),
        };
        let place = preview_place(PreviewAt::Card(rect), PANEL, WINDOW, None);
        assert!(
            place.y >= EDGE,
            "a card centred at y {cy} opened a panel at {}",
            place.y
        );
        assert!(
            place.y + PANEL.y <= WINDOW.y - HAND_ZONE_H,
            "a card centred at y {cy} opened a panel ending at {}",
            place.y + PANEL.y
        );
    }
}

/// **Never cut off**, whatever the panel and whatever the window.
///
/// Reported by the owner as the rule the placement should obey: the panel
/// adjusts itself so that it is wholly in the viewport, with some padding
/// to the edges. Every other preference here — beside the card, above the
/// hand zone, centred on the span — gives way to it.
///
/// The case the old arithmetic lost is the last row: `preview_scale` goes
/// to 1.75, which is a panel of 551 × 765, and on a 720-pixel-high window
/// that does not fit above a hand zone 174 pixels tall. The band's bounds
/// crossed, the clamp took the wrong one, and the preview hung off the
/// bottom of the screen.
#[test]
fn the_panel_is_never_cut_off_by_a_window_edge() {
    let windows = [
        Vec2::new(1728.0, 1052.0),
        Vec2::new(1280.0, 720.0),
        Vec2::new(1024.0, 640.0),
        Vec2::new(3200.0, 1738.0),
    ];
    // The scale slider's two ends, padding included.
    let panels = [
        Vec2::new(320.0, 442.0),
        Vec2::new(551.0, 765.0),
        Vec2::new(166.0, 227.0),
    ];
    for window in windows {
        for panel in panels {
            let card = |x: f32, y: f32| Rect {
                min: Vec2::new(x - 52.0, y - 73.0),
                max: Vec2::new(x + 52.0, y + 73.0),
            };
            let anchors = [
                PreviewAt::Loose,
                PreviewAt::Hand(20.0),
                PreviewAt::Hand(window.x - 20.0),
                PreviewAt::Pointer(Vec2::new(2.0, 2.0)),
                PreviewAt::Pointer(window - Vec2::splat(2.0)),
                PreviewAt::Pointer(window / 2.0),
                PreviewAt::Card(card(60.0, 60.0)),
                PreviewAt::Card(card(window.x - 60.0, window.y - 60.0)),
                PreviewAt::Card(card(window.x / 2.0, window.y / 2.0)),
            ];
            for at in anchors {
                let place = preview_place(at, panel, window, None);
                assert!(
                    place.x >= 0.0 && place.y >= 0.0,
                    "{at:?} with a {panel} panel in a {window} window opened \
                     at {place}, off the top or the left"
                );
                // A panel bigger than the window can only obey the corner
                // it starts at; every one that fits obeys both edges.
                if panel.x + 2.0 * 8.0 <= window.x && panel.y + 2.0 * 8.0 <= window.y {
                    assert!(
                        place.x + panel.x <= window.x && place.y + panel.y <= window.y,
                        "{at:?} with a {panel} panel in a {window} window \
                         opened at {place}, which hangs off the screen"
                    );
                }
            }
        }
    }
}

/// The other half of "never cut off": a preview larger than the window is
/// cut off wherever it is put, so it is not allowed to be larger.
///
/// The scale slider goes to 1.75, which asks for a picture 539 × 753 plus
/// its padding. That is more than a 720-pixel window has, and no
/// arithmetic in `preview_place` can rescue it — the size has to give,
/// keeping the card's aspect, or the player reads the wrong numbers off a
/// squashed one.
#[test]
fn a_preview_is_never_asked_to_be_bigger_than_the_window() {
    const PAD: f32 = 6.0;
    let aspect = 88.0 / 63.0;
    for window in [
        Vec2::new(1728.0, 1052.0),
        Vec2::new(1280.0, 720.0),
        Vec2::new(900.0, 600.0),
        Vec2::new(600.0, 400.0),
    ] {
        for scale in [0.5_f32, 1.0, 1.75] {
            let want = Vec2::new(308.0 * scale, 308.0 * scale * aspect);
            let got = preview_art_size(want, PAD, window);
            let panel = got + Vec2::splat(2.0 * PAD);
            assert!(
                panel.x <= window.x && panel.y <= window.y,
                "a {scale}× preview in a {window} window came out {panel}"
            );
            assert!(
                got.x <= want.x + 1e-3 && got.y <= want.y + 1e-3,
                "and it is never made *bigger* than the slider asked for"
            );
            assert!(
                (got.y / got.x - aspect).abs() < 1e-3,
                "a card's shape is not negotiable: {got}"
            );
        }
    }
    // The common case pays nothing at all.
    let want = Vec2::new(308.0, 308.0 * aspect);
    assert_eq!(
        preview_art_size(want, PAD, Vec2::new(1728.0, 1052.0)),
        want,
        "a default preview on a laptop is granted whole"
    );
}

/// And when it cannot clear the hand zone, it sits **as high as it can**
/// rather than wherever a crossed clamp lands.
///
/// A preview at a large `preview_scale` on a modest window is taller than
/// the space above the hand, so something has to give. What gives is the
/// bar, not the window edge, and the panel takes the topmost place there
/// is — which is the least of the card the hand can cover.
#[test]
fn a_panel_too_tall_for_the_band_sits_at_the_top_of_the_window() {
    let window = Vec2::new(1280.0, 720.0);
    let panel = Vec2::new(300.0, 600.0);
    assert!(
        panel.y > window.y - HAND_ZONE_H,
        "the case only exists while the panel really is too tall"
    );
    for at in [
        PreviewAt::Pointer(Vec2::new(640.0, 600.0)),
        PreviewAt::Card(Rect {
            min: Vec2::new(588.0, 527.0),
            max: Vec2::new(692.0, 673.0),
        }),
    ] {
        let place = preview_place(at, panel, window, None);
        assert!(
            (place.y - EDGE).abs() < 1e-3,
            "{at:?} opened at {place}, and the only sensible y here is {EDGE}"
        );
    }
}

/// The hand's bubble is unchanged: it sits ten pixels above the bar, so
/// the caret drawn between the two has something to bridge.
#[test]
fn a_hand_card_still_previews_above_the_hand_bar() {
    let place = preview_place(PreviewAt::Hand(864.0), PANEL, WINDOW, None);
    assert!(
        (place.y + PANEL.y - (WINDOW.y - HAND_ZONE_H - 10.0)).abs() < 1e-3,
        "the panel's bottom edge is at {}",
        place.y + PANEL.y
    );
    assert!(
        (place.x + PANEL.x / 2.0 - 864.0).abs() < 1e-3,
        "and it is centred on the card at {}",
        place.x + PANEL.x / 2.0
    );
}

/// The card underneath a copy stands beside the preview, clear of it, and
/// on the screen — wherever the preview itself ended up.
///
/// The last of those is the whole reason this is a function: a preview is
/// already placed against whichever window edge had the room, so "beside
/// it" is off the screen about half the time. Every anchor is walked
/// across the window rather than one convenient case being asserted.
#[test]
fn the_card_underneath_stands_beside_the_preview_and_on_the_screen() {
    let thumb = Vec2::new(105.0, 160.0);
    let mut on_the_left = 0;
    let mut on_the_right = 0;
    for x in [12.0_f32, 200.0, 864.0, 1400.0, 1716.0] {
        for y in [60.0_f32, 300.0, 526.0, 870.0] {
            let place = preview_place(PreviewAt::Pointer(Vec2::new(x, y)), PANEL, WINDOW, None);
            let preview = Rect::from_corners(place, place + PANEL);
            let at = underneath_place(preview, thumb, WINDOW);
            let it = Rect::from_corners(at, at + thumb);
            assert!(
                it.min.x >= 0.0 && it.min.y >= 0.0 && it.max.x <= WINDOW.x && it.max.y <= WINDOW.y,
                "at {at} it hangs off a {WINDOW} window"
            );
            assert!(
                it.max.x <= preview.min.x || it.min.x >= preview.max.x,
                "at {at} it lies over the preview at {place}"
            );
            if it.max.x <= preview.min.x {
                on_the_left += 1;
            } else {
                on_the_right += 1;
            }
            assert!(
                (it.max.y - preview.max.y).abs() < 1e-3,
                "its foot is at {} and the preview's at {}",
                it.max.y,
                preview.max.y
            );
        }
    }
    // Both flanks are reached, which is what says the branch is doing
    // something: a version that always went right would satisfy every
    // assertion above on a window this wide.
    assert!(
        on_the_left > 0 && on_the_right > 0,
        "{on_the_left}/{on_the_right}"
    );
}

/// A window with no room on either flank still puts it on the screen.
///
/// `preview_art_size` shrinks the preview to fit the window, so a phone
/// gets a panel nearly as wide as the screen and there is no "beside" to
/// be had. Overlapping is then the right answer and hanging off the edge
/// is not.
#[test]
fn a_window_with_no_room_beside_the_preview_keeps_it_on_the_screen() {
    let window = Vec2::new(390.0, 844.0);
    let panel = Vec2::new(360.0, 500.0);
    let thumb = Vec2::new(122.0, 184.0);
    let place = preview_place(PreviewAt::Loose, panel, window, None);
    let at = underneath_place(Rect::from_corners(place, place + panel), thumb, window);
    assert!(
        at.x >= 0.0 && at.x + thumb.x <= window.x,
        "at {at} it hangs off a {window} window"
    );
    assert!(at.y >= 0.0 && at.y + thumb.y <= window.y, "and vertically");
}

/// A drawer the size of a colour chooser, centred, standing on the ledge.
///
/// The measurements are the ones the collision actually happened at: the
/// chooser is centred on the middle of the shelf and grows upward out of it,
/// so it occupies the same band a hand card's preview is placed in.
fn chooser(window: Vec2) -> Rect {
    let size = Vec2::new(360.0, 180.0);
    Rect::from_center_size(
        Vec2::new(window.x / 2.0, window.y - HAND_ZONE_H - size.y / 2.0),
        size,
    )
}

/// With no drawer open, every placement is exactly what it was.
///
/// The keep-out is an `Option` and this is the arm that has to cost nothing.
/// It is asserted over all four anchors rather than one, because the dodge is
/// applied after the match and could reach any of them.
#[test]
fn no_drawer_places_a_preview_exactly_where_it_always_did() {
    let card = Rect::from_corners(Vec2::new(700.0, 300.0), Vec2::new(800.0, 440.0));
    for at in [
        PreviewAt::Hand(864.0),
        PreviewAt::Loose,
        PreviewAt::Card(card),
        PreviewAt::Pointer(Vec2::new(600.0, 500.0)),
    ] {
        let was = preview_place(at, PANEL, WINDOW, None);
        // The same call with a drawer that is nowhere near it: the arm that
        // costs nothing has to be the *overlap* and not the `Option`, or a
        // closed drawer and an open one somewhere else are two behaviours.
        let far = Rect::from_corners(Vec2::new(0.0, 0.0), Vec2::new(40.0, 40.0));
        let still = preview_place(at, PANEL, WINDOW, Some(far));
        assert!(
            (was - still).length() < 1e-3,
            "{at:?} moved from {was} to {still} for a drawer it does not touch"
        );
    }
}

/// A hand card's preview standing over an open chooser steps aside, and does
/// not step *up*.
///
/// The y is the assertion that matters. Moving the panel above the drawer is
/// the obvious dodge and is wrong twice — the drawer's height is whatever its
/// content asked for, and above the ledge is the board the preview was opened
/// to explain.
#[test]
fn a_preview_over_an_open_drawer_steps_aside_and_never_up() {
    let out = chooser(WINDOW);
    let over = preview_place(PreviewAt::Hand(WINDOW.x / 2.0), PANEL, WINDOW, None);
    assert!(
        over.x < out.max.x && out.min.x < over.x + PANEL.x,
        "the test is vacuous unless the undodged placement really overlaps: \
         {over} against {out:?}"
    );

    let place = preview_place(PreviewAt::Hand(WINDOW.x / 2.0), PANEL, WINDOW, Some(out));
    assert!(
        (place.y - over.y).abs() < 1e-3,
        "it moved up: {} against {}",
        place.y,
        over.y
    );
    assert!(
        place.x + PANEL.x <= out.min.x + 1e-3 || place.x >= out.max.x - 1e-3,
        "it still covers the drawer: {place} against {out:?}"
    );
}

/// It moves to the nearer flank, not to whichever the code tries first.
///
/// A card played from the left of the hand opens its preview on the left; a
/// dodge that always went right would carry the picture across the whole
/// window to clear a panel three hundred pixels wide.
#[test]
fn the_dodge_is_to_the_nearer_side_of_the_drawer() {
    let out = chooser(WINDOW);
    // Both cards have to *overlap* the chooser, or the case is decided before
    // the dodge is reached and the assertion below is about nothing. The first
    // draft used 520 and 1200: at 520 the panel ends at 680 and the chooser
    // begins at 684, so that case never dodged at all and the test passed
    // against a `clear_of` hard-wired to the right-hand side. These two
    // straddle the chooser's edges from inside, and the guard is what says so.
    for (x, want_left) in [(700.0_f32, true), (1030.0_f32, false)] {
        let over = preview_place(PreviewAt::Hand(x), PANEL, WINDOW, None);
        assert!(
            over.x < out.max.x && out.min.x < over.x + PANEL.x,
            "a card at {x} does not overlap the chooser undodged ({over} against {out:?}), so this case tests nothing"
        );
        let place = preview_place(PreviewAt::Hand(x), PANEL, WINDOW, Some(out));
        let left = place.x + PANEL.x <= out.min.x + 1e-3;
        assert_eq!(
            left,
            want_left,
            "a card at {x} dodged to the {} side, landing at {place}",
            if left { "left" } else { "right" }
        );
    }
}

/// A merged card's preview hangs its count badge off the card's left edge
/// (#261), and the badge lands on the screen wherever the panel does: a
/// preview opened at the window's left edge would otherwise hang the count
/// off it. And the bubble's clip lets out the whole badge, shadow included,
/// on every side it passes the card: off the prints (#274) its shadow falls
/// past the card's bottom edge too, by less than it overhangs the left, and
/// the clip's margin is one number for all four sides.
#[test]
fn a_preview_keeps_its_count_badge_on_the_screen() {
    use crate::hud::overlay::{badge_reach, place_with_badge};
    let [_, qy0, _, qy1] = baylee_client_core::cardplate::badge_quad_rect();
    let past = (-qy0).max(qy1 - baylee_client_core::cardframe::CARD_TALL);
    let over = |img_w: f32| -baylee_client_core::cardplate::badge_quad_rect()[0] * img_w;
    // The picture inside the panel, at the default scale and at the two ends
    // of the slider.
    for img_w in [154.0_f32, 308.0, 539.0] {
        let panel = Vec2::new(img_w + 12.0, img_w * 88.0 / 63.0 + 12.0);
        let reach = badge_reach(img_w);
        assert!(
            reach + 6.0 >= over(img_w) - 1e-3,
            "a card {img_w} wide: the clip lets out {reach} past the padding \
             and the badge reaches {} past the card",
            over(img_w)
        );
        assert!(
            past * img_w <= over(img_w),
            "a card {img_w} wide: the badge reaches {} past the card's top or \
             bottom, further than the {} past its left the clip is let out by",
            past * img_w,
            over(img_w)
        );
        let anchors = [
            PreviewAt::Pointer(Vec2::new(4.0, 300.0)),
            PreviewAt::Card(Rect::from_corners(
                Vec2::new(0.0, 200.0),
                Vec2::new(40.0, 256.0),
            )),
            PreviewAt::Hand(20.0),
            PreviewAt::Pointer(Vec2::new(1724.0, 300.0)),
        ];
        for at in anchors {
            let place = place_with_badge(at, panel, WINDOW, None, reach);
            let badge_left = place.x + 6.0 - over(img_w);
            assert!(
                badge_left >= 0.0,
                "{at:?}, a card {img_w} wide: the badge starts at {badge_left}, off the screen"
            );
            assert!(
                place.x + panel.x <= WINDOW.x,
                "{at:?}, a card {img_w} wide: the panel ends off the right at {}",
                place.x + panel.x
            );
        }
    }
}
