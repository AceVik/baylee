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
}

mod combat {
    use super::*;
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
        let (line, threatened) =
            incoming_line(&view, None, None, Lang::En).expect("combat is declared");
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
        let (line, threatened) =
            incoming_line(&view, None, None, Lang::En).expect("combat is declared");
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
        assert!(incoming_line(&view, None, None, Lang::En).is_none());
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
            combat_line(&interaction, &view, None, Lang::En).is_none(),
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
    /// on the slip, sixteen on the browser — with the sheet's own rounded
    /// corners cut inside the panel's. Two concentric rounded rectangles in
    /// two colours where there should be one sheet, which is what "the
    /// background still looks strange" was.
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

        for (name, source) in [
            ("the prompt slip", include_str!("overlay.rs")),
            ("the zone browser", include_str!("tray.rs")),
        ] {
            assert!(
                source.contains("sheet_surface(sheets)"),
                "{name} draws no parchment surface"
            );
            assert!(
                !source.contains(".insert(sheet("),
                "{name} wears the sheet as its own image again, which leaves \
                 its padding flat"
            );
        }
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
