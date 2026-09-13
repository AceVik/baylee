//! The HUD's tests: layout arithmetic and close-button decisions,
//! neither of which needs a window.

#[allow(clippy::wildcard_imports)]
use super::*;

mod layout {
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

    /// The bar clips its children, so its headroom is a promise about the
    /// tallest thing that can stand out of a card.
    ///
    /// A glow cut off flat along a horizontal line is not a subtle fault — it
    /// stops reading as a light and starts reading as a box drawn round the
    /// card — and the promise is made by three numbers that can each move on
    /// their own.
    #[test]
    fn the_bar_keeps_room_for_a_raised_card_and_its_glow() {
        use super::super::hand::{ARMED_RAISE, HALO_REACH, HAND_FOOTROOM};
        let room = HAND_BAR_H - HAND_CARD_H - HAND_FOOTROOM;
        assert!(
            room >= ARMED_RAISE + HALO_REACH,
            "a card raised by {ARMED_RAISE} with a halo reaching {HALO_REACH} \
             has {room} to stand in"
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
    /// `lead` is half the bar's spare room, so this grew as the hand
    /// shrank — the fewer cards left, the further from its card the bubble
    /// stood.
    #[test]
    fn the_preview_stands_on_the_card_it_describes() {
        let available = hand_available(1920.0);
        let layout = hand_layout(7, HAND_CARD_W, available);
        assert!(layout.lead > 100.0, "this window has room to centre in");
        for index in 0..7 {
            // Where the bar actually draws the card: its padding, the
            // strip's inset and margin, then the card's place in the row.
            let drawn = HAND_BAR_PAD + HAND_STRIP_INSET + layout.lead + index as f32 * layout.step;
            let middle = crate::hud::hand::hand_card_x(layout, 0.0, index);
            assert!(
                (middle - (drawn + HAND_CARD_W / 2.0)).abs() < 1e-3,
                "card {index} is drawn at {drawn} and the preview points at \
                 {middle}"
            );
        }
    }
}

/// Where the preview panel opens.
///
/// A hand card has a place in the HUD's own layout and the bubble has always
/// pointed at it. Nothing else does: a permanent is on the felt, a pile is
/// beside a mat, a stack card is in a panel that scrolls — so those anchor at
/// the pointer, and the arithmetic that keeps the panel beside the pointer
/// rather than on top of it, off the window's own top edge and off the hand
/// bar, is the whole of the placement.
/// Reported twice, as two bugs: "the hand flickers" and "cards jump on
/// hover". They are one branch — the hand scrolling a hovered card into view
/// for a pointer that was already on it.
mod hand_scroll {
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
}

mod preview_place {
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
                let place = preview_place(PreviewAt::Pointer(at), PANEL, WINDOW);
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
            let place = preview_place(at, PANEL, WINDOW);
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
    /// from under the hand bar, which the clamp above allows and this does
    /// not: a preview whose bottom half is behind the hand is a preview of a
    /// card's top half.
    #[test]
    fn a_pointer_anchor_clears_the_window_edge_and_the_hand_bar() {
        for y in [0.0_f32, 30.0, 500.0, 1000.0, 1052.0] {
            let place = preview_place(PreviewAt::Pointer(Vec2::new(600.0, y)), PANEL, WINDOW);
            assert!(place.y >= EDGE, "at y {y} the panel starts at {}", place.y);
            assert!(
                place.y + PANEL.y <= WINDOW.y - HAND_BAR_H,
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
                let place = preview_place(PreviewAt::Card(rect), PANEL, WINDOW);
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
            let place = preview_place(PreviewAt::Card(rect), PANEL, WINDOW);
            assert!(
                place.y >= EDGE,
                "a card centred at y {cy} opened a panel at {}",
                place.y
            );
            assert!(
                place.y + PANEL.y <= WINDOW.y - HAND_BAR_H,
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
    /// hand bar, centred on the span — gives way to it.
    ///
    /// The case the old arithmetic lost is the last row: `preview_scale` goes
    /// to 1.75, which is a panel of 551 × 765, and on a 720-pixel-high window
    /// that does not fit above a hand bar 174 pixels tall. The band's bounds
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
                    let place = preview_place(at, panel, window);
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

    /// And when it cannot clear the hand bar, it sits **as high as it can**
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
            panel.y > window.y - HAND_BAR_H,
            "the case only exists while the panel really is too tall"
        );
        for at in [
            PreviewAt::Pointer(Vec2::new(640.0, 600.0)),
            PreviewAt::Card(Rect {
                min: Vec2::new(588.0, 527.0),
                max: Vec2::new(692.0, 673.0),
            }),
        ] {
            let place = preview_place(at, panel, window);
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
        let place = preview_place(PreviewAt::Hand(864.0), PANEL, WINDOW);
        assert!(
            (place.y + PANEL.y - (WINDOW.y - HAND_BAR_H - 10.0)).abs() < 1e-3,
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
                let place = preview_place(PreviewAt::Pointer(Vec2::new(x, y)), PANEL, WINDOW);
                let preview = Rect::from_corners(place, place + PANEL);
                let at = underneath_place(preview, thumb, WINDOW);
                let it = Rect::from_corners(at, at + thumb);
                assert!(
                    it.min.x >= 0.0
                        && it.min.y >= 0.0
                        && it.max.x <= WINDOW.x
                        && it.max.y <= WINDOW.y,
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
        let place = preview_place(PreviewAt::Loose, panel, window);
        let at = underneath_place(Rect::from_corners(place, place + panel), thumb, window);
        assert!(
            at.x >= 0.0 && at.x + thumb.x <= window.x,
            "at {at} it hangs off a {window} window"
        );
        assert!(at.y >= 0.0 && at.y + thumb.y <= window.y, "and vertically");
    }
}

mod closing {
    use super::*;

    #[test]
    fn closing_the_duel_takes_the_overlay_with_it() {
        let mut app = App::new();
        app.init_resource::<HudRevision>()
            .add_systems(Update, despawn_overlay);
        let root = app.world_mut().spawn(HudRoot).id();
        let child = app.world_mut().spawn(Node::default()).id();
        app.world_mut().entity_mut(root).add_child(child);
        app.world_mut().resource_mut::<HudRevision>().seq = Some(7);

        app.update();

        let mut roots = app.world_mut().query_filtered::<Entity, With<HudRoot>>();
        assert_eq!(roots.iter(app.world()).count(), 0, "the root is gone");
        let mut nodes = app.world_mut().query_filtered::<Entity, With<Node>>();
        assert_eq!(
            nodes.iter(app.world()).count(),
            0,
            "and its children went with it"
        );
        assert!(
            app.world().resource::<HudRevision>().seq.is_none(),
            "a revision describing a tree that no longer exists would make the \
             next duel's first frame skip its own rebuild"
        );
    }
}

/// The preview panel is a *description* of the hovered card, and a description
/// that can take the pointer from the thing it describes is a flicker with a
/// delay fuse.
///
/// This reads the source because the fact is about a component on an entity
/// tree that only exists inside a running renderer, and the alternative — a
/// headless `App` with the picking backends and a window — would test Bevy's
/// wiring rather than ours. The same shape as the shader tests: the rule is
/// written once, and a test fails when the code stops saying it.
mod preview {
    /// A board card's preview opens centred on the window, which is exactly
    /// where a player's own lands are drawn. When the panel was pickable it
    /// landed on the pointer that opened it, and the card blinked; with the
    /// pointer at rest the grace window swallowed the only `Out` there would
    /// ever be and the hover latched for the rest of the game.
    ///
    /// Recursive because `Pickable` does not inherit and the card face has
    /// children of its own. The resize handle hangs off the tooltip rather
    /// than the frame, which is what keeps it clickable.
    #[test]
    fn the_hover_preview_cannot_take_the_pointer_from_the_card_it_describes() {
        let source = include_str!("overlay.rs");
        let needle = ".insert_recursive::<Children>(Pickable::IGNORE);";
        assert!(
            source.contains(needle),
            "the preview's frame must hand `Pickable::IGNORE` to its whole \
             subtree; without it the panel covers the card it is describing"
        );
    }
}

/// The redraw gate has to read every field it carries.
mod revision {
    /// A field of [`HudRevision`](crate::hud::HudRevision) that is compared
    /// but never assigned redraws the tree on every frame; one that is
    /// assigned but never compared is state that changes with nothing
    /// happening on screen.
    ///
    /// The second is not hypothetical. `choice` — which entry of the answer
    /// chooser is picked — was drawn and never gated, and picking one never
    /// leaves the client until Confirm, so nothing else in the struct moved
    /// and the brass highlight stayed on whichever entry it had been on when
    /// the tree was last built for some other reason. There was no way to
    /// find that by reading the struct, because the struct looked complete.
    ///
    /// Source-reading, for the reason the preview test above gives: the fact
    /// is about a `Res<HudRevision>` inside a running renderer, and the
    /// alternative is an `App` with a window in it.
    #[test]
    fn every_field_of_the_revision_is_both_compared_and_assigned() {
        let hud = include_str!("../hud.rs");
        let overlay = include_str!("overlay.rs");

        let body = hud
            .split_once("pub struct HudRevision {")
            .expect("the struct is still called that")
            .1;
        let body = body.split_once("\n}").expect("and still closes").0;
        let fields: Vec<&str> = body
            .lines()
            .filter_map(|line| {
                let name = line.strip_prefix("    ")?;
                let (name, _) = name.split_once(':')?;
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                    .then_some(name)
            })
            .collect();
        assert!(fields.len() > 15, "the fields did not parse: {fields:?}");

        let writes = overlay
            .find("revision.seq = seq;")
            .expect("the assignment block is still written out field by field");
        let (gate, assign) = overlay.split_at(writes);
        // From the anchor itself, not past it: `seq` is the field the gate
        // opens with.
        let opens = gate
            .rfind("if revision.seq == seq")
            .expect("and the gate above it");
        let gate = &gate[opens..];

        for field in fields {
            let needle = format!("revision.{field}");
            assert!(
                gate.contains(&needle),
                "`{field}` is remembered but never compared: it can change \
                 with nothing on screen changing"
            );
            assert!(
                assign.contains(&needle),
                "`{field}` is compared but never written, so the tree rebuilds \
                 every frame the moment it differs once"
            );
        }
    }

    /// The blind spot the test above has, one level down.
    ///
    /// `browser` passed that test for as long as it existed, because the
    /// whole field was compared and the whole field was assigned. It was a
    /// four-tuple, and the browser has **six** things a player can move: the
    /// sort key and its direction were not in it, so clicking either of the
    /// two buttons beside the filter box changed the order of a list that was
    /// never redrawn. Both controls did nothing at all on screen.
    ///
    /// A struct with named fields is most of the fix — a literal that names
    /// every field cannot forget one and still compile — so what is left to
    /// guard is the escape hatch: `..Default::default()` would put the hole
    /// straight back, with the compiler content.
    #[test]
    fn the_browsers_gate_is_filled_field_by_field() {
        let hud = include_str!("../hud.rs");
        let overlay = include_str!("overlay.rs");

        let body = hud
            .split_once("struct BrowserGate {")
            .expect("the gate is still its own struct")
            .1
            .split_once("\n}")
            .expect("and still closes")
            .0;
        let fields: Vec<&str> = body
            .lines()
            .filter_map(|line| {
                let (name, _) = line.strip_prefix("    ")?.split_once(':')?;
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                    .then_some(name)
            })
            .collect();
        assert_eq!(fields.len(), 6, "the fields did not parse: {fields:?}");

        let built = overlay
            .split_once("let browser = BrowserGate {")
            .expect("still built where the gate is assembled")
            .1
            .split_once("\n    };")
            .expect("and still closes")
            .0;
        for field in fields {
            assert!(
                built.contains(&format!("{field}:")),
                "`{field}` is in the gate and is never read off the browser"
            );
        }
        assert!(
            !built.contains(".."),
            "a struct update fills the rest from `Default`, which is the hole \
             the named fields were supposed to close"
        );
    }
}

mod combat {
    use super::*;
    use crate::cardtext::CardTexts;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::ids::Defender;
    use baylee_view::{AttackerView, BlockerView};

    fn attacked_by(power: i16, blocked: bool) -> PlayerView {
        let blockers = if blocked {
            vec![BlockerView {
                blocker: ObjectId::new(10, 0),
                attacker: ObjectId::new(1, 0),
            }]
        } else {
            Vec::new()
        };
        ViewBuilder::new(2)
            .with_battlefield(1, vec![token(1, 1, "Ogre", power, 3)])
            .with_battlefield(0, vec![token(10, 0, "Wall", 0, 4)])
            .with_combat(
                vec![AttackerView {
                    creature: ObjectId::new(1, 0),
                    defending: Defender::Player(PlayerId::new(0)),
                    blocked,
                }],
                blockers,
            )
            .build()
    }

    #[test]
    fn an_attack_aimed_at_this_seat_is_read_out_and_marked() {
        let view = attacked_by(3, false);
        let (line, threatened) = incoming_line(&view, None, None, &CardTexts::default(), Lang::En)
            .expect("combat is declared");
        assert!(
            line.contains('3'),
            "the number that gets through is in the line: {line}"
        );
        assert!(
            threatened,
            "three unblocked power at this seat is worth a colour"
        );
    }

    #[test]
    fn a_blocked_attack_is_still_read_out_but_no_longer_marked() {
        let view = attacked_by(3, true);
        let (line, threatened) = incoming_line(&view, None, None, &CardTexts::default(), Lang::En)
            .expect("combat is declared");
        assert!(
            !threatened,
            "nothing reaches this seat once the attacker is blocked: {line}"
        );
    }

    #[test]
    fn there_is_no_line_when_nobody_is_attacking() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![token(1, 0, "Bear", 2, 2)])
            .build();
        assert!(incoming_line(&view, None, None, &CardTexts::default(), Lang::En).is_none());
    }

    #[test]
    fn the_aim_line_says_nothing_about_a_card_choice() {
        // A target prompt has an aim too, and it is aimed at the candidate a
        // click would pick rather than at a defender — so `combat_focus` has
        // nothing to name and this line, drawn anyway, would say "aiming at
        // nothing (1 of 3)" every time a spell asked which card to discard.
        let view = ViewBuilder::new(2).build();
        let choice = baylee_engine::choice::Pending::ChooseCards {
            player: PlayerId::new(0),
            options: vec![
                ObjectId::new(1, 0),
                ObjectId::new(2, 0),
                ObjectId::new(3, 0),
            ],
            min: 1,
            max: 1,
            prompt: baylee_engine::choice::ChoicePrompt::SearchLibrary,
        };
        let interaction = baylee_client_core::Interaction::new(choice, PlayerId::new(0));
        assert!(interaction.focus_position().is_some(), "there is an aim");
        assert!(
            combat_line(&interaction, &view, None, &CardTexts::default(), Lang::En).is_none(),
            "but it is not a combat aim, and this line only speaks for combat"
        );
    }
}

/// What shift turns a preview over to.
mod flip {
    use baylee_client_core::images::{ArtSize, Face, ImageKey};
    use baylee_core::ids::PrintRef;

    /// Shift used to do nothing on nine cards out of ten: the far side was
    /// built only for a printing with two faces, so a gesture that works on a
    /// Delver of Secrets and not on a Mountain reads as broken rather than as
    /// inapplicable. Every card has a back, and it is the same back.
    #[test]
    fn an_ordinary_card_turns_over_to_the_printed_back() {
        let key = ImageKey::new(PrintRef::new(3), 0, ArtSize::Normal);
        assert_eq!(
            crate::hud::overlay::far_face(Some(key), false),
            None,
            "a single-faced printing has nothing on its far side but the back"
        );
    }

    #[test]
    fn a_double_faced_card_still_turns_over_to_its_second_face() {
        let key = ImageKey::new(PrintRef::new(3), 0, ArtSize::Normal);
        let far = crate::hud::overlay::far_face(Some(key), true).expect("the second face");
        assert_eq!(far.face, Face::Back);
        assert_eq!(far.source, key.source, "the same printing, turned over");
        assert_eq!(far.size, key.size);
    }

    /// The two faces are one card seen from two sides, so they stand in the
    /// same place. Laid out in the frame's flow they were two items in a row
    /// instead, and taffy shrank each to half the frame: the preview drew a
    /// card squeezed into its left half with the hidden face's empty slot
    /// beside it, on every card in the client.
    #[test]
    fn both_faces_of_the_preview_stand_in_the_same_place() {
        let node = crate::hud::overlay::face_node(190.0, 265.0);
        assert_eq!(
            node.position_type,
            bevy::ui::PositionType::Absolute,
            "a face in the frame's flow is an item in a row, and two of them share the width"
        );
        assert_eq!(node.width, bevy::ui::Val::Px(190.0));
        assert_eq!(node.height, bevy::ui::Val::Px(265.0));
    }

    /// A card this seat may not read has no printing to ask about — and it is
    /// precisely the card whose back is the interesting side, because the
    /// back is all anyone else at the table can see of it.
    #[test]
    fn a_card_with_no_printing_turns_over_to_the_back_as_well() {
        assert_eq!(crate::hud::overlay::far_face(None, false), None);
        assert_eq!(crate::hud::overlay::far_face(None, true), None);
    }
}

mod slip {
    use super::*;

    /// The one question a player must answer stands in the middle, not in a
    /// corner.
    ///
    /// It shipped in the bottom-right at 13 px against 88% black — the least
    /// prominent thing on screen, in the corner furthest from the hand it is
    /// answered from. This is the claim that stops it drifting back there:
    /// the row spans the window and centres what is in it, and it clears the
    /// hand bar rather than sitting behind it.
    #[test]
    fn the_prompt_slip_stands_in_the_middle_above_the_hand() {
        let node = super::super::overlay::slip_row_node();
        assert_eq!(node.justify_content, JustifyContent::Center);
        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!(node.left, px(0), "a row that does not span cannot centre");
        assert_eq!(node.right, px(0));
        let Val::Px(bottom) = node.bottom else {
            panic!("the slip is placed in pixels, not {:?}", node.bottom);
        };
        assert!(
            bottom > HAND_BAR_H && bottom < HAND_BAR_H + 60.0,
            "the slip sits at {bottom}, and the hand bar is {HAND_BAR_H} tall — \
             it has to clear it and stay next to it"
        );
    }

    /// The parchment covers the padding it lies under.
    ///
    /// [`sheet`] inserted on a panel paints that panel's **content box**, so
    /// a sheet with padding drew the grain in the middle and flat
    /// [`palette::PARCHMENT`] in a ring around it — twenty-two pixels of it
    /// on the slip, sixteen on the zone browser it was then — with the
    /// sheet's own rounded corners cut inside the panel's. Two concentric
    /// rounded rectangles in two colours where there should be one sheet,
    /// which is what "the background still looks strange" was.
    ///
    /// The browser is a dark panel now and carries no parchment at all
    /// (`docs/redesign-proposal.md` §1.3), so the slip is the one surface
    /// left that this is about.
    ///
    /// An absolutely-positioned child is measured against its parent's
    /// *padding* box, which is exactly the missing ring. Both halves are
    /// asserted: that the surface is placed that way, and that neither panel
    /// has gone back to wearing the sheet itself.
    #[test]
    fn the_parchment_covers_the_padding_it_lies_under() {
        let sheets = UiSheets {
            parchment: Handle::default(),
        };
        let mut app = App::new();
        let surface = app.world_mut().spawn(sheet_surface(&sheets)).id();
        let node = app
            .world()
            .entity(surface)
            .get::<Node>()
            .expect("the surface is a node");
        assert_eq!(
            node.position_type,
            PositionType::Absolute,
            "a surface in the flow takes a row of the column it is meant to \
             lie under"
        );
        for (side, val) in [
            ("left", node.left),
            ("right", node.right),
            ("top", node.top),
            ("bottom", node.bottom),
        ] {
            assert_eq!(val, px(0), "the sheet stops short of the {side} edge");
        }
        assert!(
            app.world().entity(surface).contains::<ImageNode>(),
            "there is no parchment on it"
        );

        let slip = include_str!("overlay.rs");
        assert!(
            slip.contains("sheet_surface(sheets)"),
            "the prompt slip draws no parchment surface"
        );
        assert!(
            !slip.contains(".insert(sheet("),
            "the prompt slip wears the sheet as its own image again, which \
             leaves its padding flat"
        );
        // And the browser stays a panel: a sheet put back on it is the
        // material decision of §1.3 being undone by accident.
        assert!(
            !include_str!("tray.rs").contains("sheet_surface("),
            "the zone browser is parchment again, and it is a place you work"
        );
    }

    /// The answers divide the sheet between them.
    ///
    /// The claim the owner asked for — "100% width, evenly distributed, with
    /// a small padding between them" — and the reason it needs a test is the
    /// `flex_basis`: `flex_grow: 1.0` on its own divides only the slack left
    /// after the labels, so three answers with three different words still
    /// come out three different widths. Zero is what takes the labels out of
    /// the sum.
    #[test]
    fn every_answer_is_drawn_the_same_width_as_every_other() {
        let row = super::super::overlay::answer_row_node();
        assert_eq!(
            row.width,
            percent(100),
            "a row that does not span cannot share"
        );
        assert_eq!(row.flex_direction, FlexDirection::Row);
        let Val::Px(gap) = row.column_gap else {
            panic!("the gap is in pixels, not {:?}", row.column_gap);
        };
        assert!(
            gap > 0.0 && gap < 20.0,
            "the answers are {gap} apart, which is not the small padding asked for"
        );

        let button = super::super::overlay::answer_node();
        assert!((button.flex_grow - 1.0).abs() < f32::EPSILON);
        assert_eq!(
            button.flex_basis,
            px(0),
            "a basis that is not zero leaves the label in the sum, and the \
             widths follow the words instead of the row"
        );
        assert_eq!(button.justify_content, JustifyContent::Center);
    }
}

/// The rail's light is *run*, not merely declared.
///
/// The step a game is in is drawn by a system rather than by a colour written
/// when the button is built, so the thing that can go wrong is the system
/// never being scheduled — a class of bug this client has shipped before, and
/// which every assertion about the colour it *would* write would have missed.
/// So the claim here is about an `App` that has actually run: the light rises
/// from nothing, and it rises towards the colour a player is meant to read as
/// "here".
mod the_current_step {
    use super::*;

    fn lit(app: &App, button: Entity) -> f32 {
        app.world()
            .entity(button)
            .get::<PhaseNow>()
            .expect("the button still carries its light")
            .lit
    }

    #[test]
    fn the_light_arrives_over_several_frames_rather_than_cutting() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_systems(Update, light_the_current_step);
        let button = app
            .world_mut()
            .spawn((
                PhaseNow::default(),
                BorderColor::all(palette::PANEL),
                BoxShadow::new(Color::NONE, px(0), px(0), px(0), px(0)),
            ))
            .id();
        assert!(lit(&app, button).abs() < 1e-6, "it starts dark");

        // A frame at a time, because the ease is exponential: the first frame
        // must land somewhere between the two ends, which is the whole claim.
        // A `Time` with no delta would satisfy "not one" by never moving at
        // all, so the delta is written by hand.
        let frame = std::time::Duration::from_millis(16);
        app.world_mut().resource_mut::<Time>().advance_by(frame);
        app.update();
        let first = lit(&app, button);
        assert!(
            first > 0.0 && first < 1.0,
            "one frame took the light from 0 to {first}"
        );

        for _ in 0..80 {
            app.world_mut().resource_mut::<Time>().advance_by(frame);
            app.update();
        }
        assert!(
            (lit(&app, button) - 1.0).abs() < 1e-6,
            "the light never finished arriving: {}",
            lit(&app, button)
        );
        let border = app
            .world()
            .entity(button)
            .get::<BorderColor>()
            .expect("set");
        assert_eq!(
            border.top,
            palette::ACTIVE,
            "the step the game is in is not drawn in the colour that says so"
        );
        let shadow = app.world().entity(button).get::<BoxShadow>().expect("set");
        assert!(
            shadow.first().is_some_and(|s| s.color.alpha() > 0.0),
            "and it casts no light at all"
        );
    }
}

/// The day/night block: when it exists at all, and the flash that marks a
/// change surviving a rebuild.
mod the_designation {
    use super::*;
    use baylee_view::DayNight;

    fn glow(app: &App, block: Entity) -> f32 {
        app.world()
            .entity(block)
            .get::<BoxShadow>()
            .and_then(|s| s.first().map(|l| l.color.alpha()))
            .expect("the block still carries its light")
    }

    fn spawn(app: &mut App, now: DayNight) -> Entity {
        app.world_mut()
            .spawn((
                Designation(now),
                BorderColor::all(Color::NONE),
                BoxShadow::new(Color::NONE, px(0), px(0), px(0), px(0)),
            ))
            .id()
    }

    fn frame(app: &mut App, ms: u64) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(ms));
        app.update();
    }

    /// The flash is anchored to the *change*, not to the entity, and that is
    /// the whole design: the HUD tree is rebuilt on every hover, so a light
    /// that eased from zero at spawn would fire again every time the pointer
    /// crossed a card. Here the block is despawned and respawned with the
    /// same designation — a rebuild, exactly — and the decay has to carry on
    /// from where it was rather than start over.
    #[test]
    fn a_rebuild_does_not_restart_the_flash() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<DesignationFlash>()
            .add_systems(Update, flash_the_designation);

        let first = spawn(&mut app, DayNight::Day);
        frame(&mut app, 16);
        let arrival = glow(&app, first);
        assert!(
            arrival > 0.4,
            "the block did not announce itself: {arrival}"
        );

        frame(&mut app, 400);
        let decayed = glow(&app, first);
        assert!(
            decayed < arrival * 0.5,
            "the flash did not decay: {arrival} -> {decayed}"
        );

        // The rebuild.
        app.world_mut().entity_mut(first).despawn();
        let second = spawn(&mut app, DayNight::Day);
        frame(&mut app, 16);
        let after = glow(&app, second);
        assert!(
            after <= decayed,
            "the rebuild restarted the flash: {decayed} -> {after}"
        );
    }

    /// And the counter-test, or the one above would pass just as well on a
    /// system that never lit anything after the first frame: a block that
    /// comes back carrying the *other* designation is a change, and a change
    /// flashes.
    #[test]
    fn the_other_designation_is_a_change_and_flashes() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<DesignationFlash>()
            .add_systems(Update, flash_the_designation);

        let day = spawn(&mut app, DayNight::Day);
        frame(&mut app, 16);
        frame(&mut app, 800);
        let quiet = glow(&app, day);
        assert!(quiet < 0.1, "the flash never settled: {quiet}");

        app.world_mut().entity_mut(day).despawn();
        let night = spawn(&mut app, DayNight::Night);
        frame(&mut app, 16);
        assert!(
            glow(&app, night) > 0.4,
            "night arrived without saying so: {}",
            glow(&app, night)
        );
    }

    /// The designation's two glyphs belong to nothing else on a seat's bar.
    ///
    /// The sun was the untap step's until the day designation wanted it, and
    /// two suns a hundred pixels apart on one bar would have said the untap
    /// step *is* the daytime. The check is the two sets being disjoint and
    /// not every glyph being unique, because the two main phases share a flag
    /// on purpose — they are one phase kind twice, and "M1" and "M2" under
    /// them are what tell them apart.
    ///
    /// The two halves used to live in one file and the test split it at
    /// `fn spawn_designation`. They are two files now — the steps' glyphs
    /// stayed with [`row_visual`](crate::hud::rail) when the rail went, and
    /// the hinge that carries the designation is on the bar — so the test
    /// reads both instead of splitting one.
    #[test]
    fn the_designation_does_not_borrow_a_step_glyph() {
        let glyphs = |src: &str| -> Vec<String> {
            src.match_indices("'\\u{f")
                .map(|(at, _)| src[at + 1..at + 9].to_string())
                .collect()
        };
        let steps = glyphs(
            include_str!("rail.rs")
                .split_once("fn row_visual")
                .expect("the steps' glyphs are still there")
                .1
                .split_once("\n}")
                .expect("and the table still closes")
                .0,
        );
        let bar = include_str!("seatbar.rs");
        let designation = glyphs(
            bar.split_once("fn designation_of")
                .expect("the hinge still names its two glyphs")
                .1
                .split_once("\n}")
                .expect("and still closes")
                .0,
        );
        assert_eq!(
            designation.len(),
            2,
            "the sun and the moon: {designation:?}"
        );
        assert_eq!(steps.len(), 12, "the rows did not parse: {steps:?}");
        for glyph in &designation {
            assert!(
                !steps.contains(glyph),
                "{glyph} is drawn both as a step and as the designation"
            );
        }
    }
}

/// A split bar's tiles are the one ink on any bar whose width is not a fixed
/// number of pixels, and the tree they live in is built only when the
/// *density* changes. So the width they are born with is the length the ledge
/// projected at that moment, and the camera is still easing towards its home
/// then — every duel opened with a bar whose box covered the whole shelf and
/// whose tiles covered nine tenths of it, the difference going quietly into
/// the phase gaps. The claim here is about an `App` that has actually run: a
/// shelf that grows takes its tiles with it.
mod the_tiles_follow_the_shelf {
    use super::*;
    use crate::hud::{SeatTile, Shelf, Shelves, stretch_step_tiles};
    use baylee_client_core::seatbar::Density;
    use baylee_core::ids::PlayerId;

    const SEAT: PlayerId = PlayerId::new(0);

    /// How long a duel's near shelf projects while the camera is still on its
    /// way in, and how long it is once it has arrived. Both measured on the
    /// running client at 1728x1052.
    const ARRIVING: f32 = 981.0;
    const HOME: f32 = 1127.1;

    #[derive(Resource, Default)]
    struct Relayouts(usize);

    fn count(mut seen: ResMut<Relayouts>, tiles: Query<(), Changed<Node>>) {
        seen.0 += tiles.iter().count();
    }

    fn shelf(along: f32) -> Shelf {
        Shelf {
            middle: Vec2::new(864.0, 479.0),
            along,
            depth: 61.0,
            tilt: 0.0,
            density: Density::Split,
        }
    }

    fn width(app: &App, tile: Entity) -> f32 {
        match app
            .world()
            .entity(tile)
            .get::<Node>()
            .expect("a node")
            .width
        {
            Val::Px(w) => w,
            other => panic!("a tile is sized in pixels, not {other:?}"),
        }
    }

    /// A bar as the tree builder leaves it: one ordinary step and one main
    /// phase, both born on the shorter shelf.
    fn table(along: f32) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<crate::Duel>()
            .init_resource::<Relayouts>()
            .insert_resource(Shelves(vec![(SEAT, shelf(along))]))
            .add_systems(Update, (stretch_step_tiles, count).chain());
        let born = Density::Split.tile_width_on(along);
        let mut tile = |span: f32| {
            app.world_mut()
                .spawn((
                    SeatTile { player: SEAT, span },
                    Node {
                        width: px(born * span),
                        ..default()
                    },
                ))
                .id()
        };
        let step = tile(1.0);
        let main = tile(Density::Split.main_span());
        (app, step, main)
    }

    #[test]
    fn a_camera_that_dollies_in_widens_the_tiles_with_the_shelf() {
        let (mut app, step, _) = table(ARRIVING);
        app.update();
        let born = width(&app, step);

        app.insert_resource(Shelves(vec![(SEAT, shelf(HOME))]));
        app.update();

        let want = Density::Split.tile_width_on(HOME);
        assert!(
            (width(&app, step) - want).abs() < 1e-3,
            "the shelf grew from {ARRIVING} to {HOME} and the tile stayed at \
             {born}: it is {} where the model says {want}",
            width(&app, step)
        );
        // And the counter-test the bug would have passed: the tile really did
        // have to move. A system that wrote nothing at all would agree with
        // the model here if the model happened to answer the same twice.
        assert!(
            want > born,
            "the two shelves have to disagree or this proves nothing: \
             {born} at {ARRIVING}, {want} at {HOME}"
        );
    }

    #[test]
    fn a_main_phase_grows_by_its_own_span() {
        let (mut app, step, main) = table(ARRIVING);
        app.insert_resource(Shelves(vec![(SEAT, shelf(HOME))]));
        app.update();

        let span = Density::Split.main_span();
        assert!(
            (width(&app, main) - width(&app, step) * span).abs() < 1e-3,
            "a main phase is {span} steps wide wherever the shelf is: \
             {} against {}",
            width(&app, main),
            width(&app, step)
        );
    }

    /// Touching a `Node` at all relays out the bar it belongs to, so a camera
    /// standing still — which is most frames — has to cost nothing.
    #[test]
    fn a_shelf_that_has_not_moved_writes_nothing() {
        let (mut app, _, _) = table(HOME);
        app.update();
        let settled = app.world().resource::<Relayouts>().0;

        app.update();
        assert_eq!(
            app.world().resource::<Relayouts>().0,
            settled,
            "a still camera relaid out the tiles anyway"
        );

        app.insert_resource(Shelves(vec![(SEAT, shelf(ARRIVING))]));
        app.update();
        assert_eq!(
            app.world().resource::<Relayouts>().0,
            settled + 2,
            "and a shelf that did move has to reach both tiles"
        );
    }

    /// A bar the camera cannot see is hidden rather than despawned, and its
    /// tiles keep what they had: the shelf comes back at the length it went
    /// away with far more often than not, and a tile written to some fallback
    /// width would be wrong for the frame the bar is shown again.
    #[test]
    fn a_shelf_that_is_gone_leaves_its_tiles_alone() {
        let (mut app, step, _) = table(HOME);
        app.update();
        let held = width(&app, step);

        app.insert_resource(Shelves(Vec::new()));
        app.update();
        assert!(
            (width(&app, step) - held).abs() < 1e-3,
            "a hidden bar had its tiles rewritten: {held} became {}",
            width(&app, step)
        );
    }
}

/// The faces on disk, read as files rather than as handles.
///
/// `AssetServer::load` is lazy and infallible: a name that matches nothing
/// hands back a handle that never resolves, so a renamed or forgotten `.ttf`
/// is an interface drawn in *nothing* and a test suite that says so
/// nowhere. These read the bytes.
mod faces {
    /// Every face [`super::setup_fonts`] asks for, in its own spelling.
    ///
    /// Typed out a second time on purpose. The point is to hold the loader's
    /// strings against the directory, and a list built from the loader could
    /// only ever agree with it.
    const SHIPPED: [&str; 8] = [
        "AlegreyaSans-Regular.ttf",
        "AlegreyaSans-Medium.ttf",
        "AlegreyaSans-Bold.ttf",
        "AlegreyaSans-Italic.ttf",
        "AlegreyaSans-MediumItalic.ttf",
        "Faustina.ttf",
        "Faustina-Italic.ttf",
        "fa-solid-900.ttf",
    ];

    fn path(file: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/fonts")
            .join(file)
    }

    /// A TrueType table's offset and length, by tag.
    ///
    /// Twenty lines of hand-rolled parser rather than a dependency, for the
    /// same reason the sounds are computed: this reads two numbers out of one
    /// table and a font crate in the tree would be a build cost paid on every
    /// machine forever.
    fn table(bytes: &[u8], tag: [u8; 4]) -> Option<(usize, usize)> {
        let count = u16::from_be_bytes([*bytes.get(4)?, *bytes.get(5)?]) as usize;
        (0..count).find_map(|i| {
            let at = 12 + 16 * i;
            (bytes.get(at..at + 4)? == tag).then(|| {
                let n = |o: usize| {
                    u32::from_be_bytes([
                        bytes[at + o],
                        bytes[at + o + 1],
                        bytes[at + o + 2],
                        bytes[at + o + 3],
                    ]) as usize
                };
                (n(8), n(12))
            })
        })
    }

    /// `OS/2`'s `usWeightClass`: 400 for a Regular, 700 for a Bold.
    fn weight(bytes: &[u8]) -> Option<u16> {
        let (at, len) = table(bytes, *b"OS/2")?;
        (len >= 6).then(|| u16::from_be_bytes([bytes[at + 4], bytes[at + 5]]))
    }

    /// Every face the loader names is a file that is there.
    #[test]
    fn every_face_the_client_asks_for_is_in_the_tree() {
        for file in SHIPPED {
            let at = path(file);
            let bytes =
                std::fs::read(&at).unwrap_or_else(|_| panic!("{} is not shipped", at.display()));
            assert!(bytes.len() > 1024, "{file} is {} bytes", bytes.len());
            assert!(
                table(&bytes, *b"OS/2").is_some(),
                "{file} has no OS/2 table, so it is not a font this reads"
            );
        }
    }

    /// The bold cut is bold, and the other two weights of the family are not.
    ///
    /// The whole of `tf_bold` rests on a *file*, because `TextFont::weight`
    /// reaches only a variable font and these are static cuts. A file that
    /// was quietly the Regular under a bold name would draw every control in
    /// this client at the weight it had before, and nothing else in the suite
    /// could tell.
    #[test]
    fn the_bold_cut_is_the_one_that_is_bold() {
        let of =
            |file: &str| weight(&std::fs::read(path(file)).expect("a face")).expect("a weight");
        assert_eq!(of("AlegreyaSans-Bold.ttf"), 700, "the bold cut is not bold");
        assert_eq!(of("AlegreyaSans-Medium.ttf"), 500);
        assert_eq!(of("AlegreyaSans-Regular.ttf"), 400);
    }

    /// Nothing under `assets/fonts` is unaccounted for.
    ///
    /// `docs/legal.md` is the reason this is worth a test rather than a
    /// glance: every file in there is a third party's, licensed under a
    /// licence that has to be vendored beside it, and a face that arrived
    /// without being named anywhere is exactly the kind of thing an audit is
    /// supposed to find. The mana font is named by clause 2 and loaded
    /// through `manaui`, so it is listed here as shipped-and-known.
    #[test]
    fn no_face_is_shipped_that_nothing_names() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fonts");
        let mut found: Vec<String> = std::fs::read_dir(&dir)
            .expect("the font directory")
            .filter_map(|entry| {
                let name = entry.ok()?.file_name().to_string_lossy().into_owned();
                std::path::Path::new(&name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("ttf"))
                    .then_some(name)
            })
            .collect();
        found.sort();
        let mut known: Vec<String> = SHIPPED
            .iter()
            .map(|s| (*s).to_string())
            .chain(std::iter::once("mana.ttf".to_string()))
            .collect();
        known.sort();
        assert_eq!(found, known, "a face nothing in the client names");
    }
}
