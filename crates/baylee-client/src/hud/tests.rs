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
}
