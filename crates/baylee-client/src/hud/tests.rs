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
}

/// Where the preview panel opens.
///
/// A hand card has a place in the HUD's own layout and the bubble has always
/// pointed at it. Nothing else does: a permanent is on the felt, a pile is
/// beside a mat, a stack card is in a panel that scrolls — so those anchor at
/// the pointer, and the arithmetic that keeps the panel beside the pointer
/// rather than on top of it, off the tab strip, off the hand bar and clear of
/// the phase rail is the whole of the placement.
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
    /// window a player can see it in: the tab strip is above, the hand bar
    /// below, the phase rail to the right.
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
                place.x + PANEL.x <= WINDOW.x - rail::RAIL_W,
                "{at:?} put the panel under the phase rail: {place}"
            );
            assert!(place.y >= 0.0, "{at:?} put the panel off the top: {place}");
            assert!(
                place.y + PANEL.y <= WINDOW.y,
                "{at:?} put the panel off the bottom: {place}"
            );
        }
    }

    /// A pointer anchor is also kept out from under the two bars, which the
    /// clamp above allows and this does not: a preview whose top half is
    /// behind the tab strip is a preview of a card's bottom half.
    #[test]
    fn a_pointer_anchor_clears_the_tab_strip_and_the_hand_bar() {
        for y in [0.0_f32, 30.0, 500.0, 1000.0, 1052.0] {
            let place = preview_place(PreviewAt::Pointer(Vec2::new(600.0, y)), PANEL, WINDOW);
            assert!(place.y >= TAB_H, "at y {y} the panel starts at {}", place.y);
            assert!(
                place.y + PANEL.y <= WINDOW.y - HAND_BAR_H,
                "at y {y} the panel ends at {}",
                place.y + PANEL.y
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

mod own_board {
    use super::*;

    /// A closed overlay shows its handle, and nothing else.
    ///
    /// The panel's height is `window - top - HAND_BAR_H`, so closing it leaves
    /// exactly the knob standing above the hand bar — and a card inside it is
    /// an order of magnitude taller than that. Found in a photograph: the tops
    /// of two permanents stood above the hand bar, clipped to their title
    /// bars, and looked exactly like the two cards that had just been played.
    #[test]
    fn a_closed_overlay_is_no_taller_than_its_knob() {
        for window_h in [720.0_f32, 1052.0, 1138.0, 2160.0] {
            let top = overlay::closed_overlay_top(window_h);
            let height = window_h - top - HAND_BAR_H;
            // A tolerance rather than an exact compare: the height is a
            // round trip through a window height in the hundreds, and
            // `f32::EPSILON` is the spacing at 1.0, not at 720.
            assert!(
                (height - overlay::KNOB_H).abs() < 1e-3,
                "at {window_h} px the closed panel is {height} px, not the knob's \
                 {}",
                overlay::KNOB_H,
            );
        }
        // A compile-time check, because both sides are constants: if a card
        // ever fits inside the closed panel this test is measuring nothing,
        // and the clip stops being the thing that matters.
        const {
            assert!(OVERLAY_CARD_H > overlay::KNOB_H);
        }
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
        app.world_mut().resource_mut::<HudRevision>().overlay_open = true;

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
            !app.world().resource::<HudRevision>().overlay_open,
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
}
