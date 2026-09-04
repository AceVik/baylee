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
