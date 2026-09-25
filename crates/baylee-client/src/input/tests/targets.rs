//! Choosing an object the engine named, and reaching it with either hand. A `ChooseTargets` enumerates its options, so a click on anything outside that list is nothing; the harder half is that every surface those options are drawn on can be answered at all — the stack panel is where a counterspell's only option lives, and the pointer and the card cursor both once walked past it. What is proved here goes through the real `pointer` system or through `cursor_grid`/`move_cursor`, never through a hand-called `toggle`, because the model always allowed the selection and it was the way to *say* it that was missing. The press-and-release plumbing underneath a click belongs in `taps`, and what a tap on a card then does in `arming`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn a_click_on_something_the_engine_did_not_offer_does_nothing() {
    let mut i = Interaction::new(
        Pending::ChooseTargets {
            player: PlayerId::new(0),
            options: vec![obj(1)],
            player_options: vec![],
            min: 1,
            max: 1,
            reason: baylee_engine::choice::TargetPrompt::Targets,
        },
        PlayerId::new(0),
    );
    i.toggle(obj(99));
    assert!(i.selected().next().is_none());
    assert!(!i.can_confirm());
}

/// A counterspell has to be able to name the spell it counters.
///
/// A live duel stopped dead on a `ChooseTargets { options: [200], min: 1 }`
/// whose only option was a spell on the stack. The model had always
/// allowed it — `Interaction::toggle` takes "a permanent, a card in a
/// zone, or a spell on the stack" — and there was no way to *say* it: the
/// stack panel's one pickable node was the 66-pixel picture inside the
/// row, every other node carried `Pickable::IGNORE`, and a question with
/// a minimum of one cannot be passed. The row carries `HandCardVisual`
/// now, so this goes through the real `pointer` system and the real
/// message rather than calling `toggle` by hand — which is the only way
/// the test can fail if the wiring is undone again.
#[test]
fn a_click_on_a_stack_row_answers_the_question_it_was_asked() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::touch::Touched>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Click>>()
        .insert_resource(crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                Pending::ChooseTargets {
                    player: PlayerId::new(0),
                    options: vec![obj(200)],
                    player_options: vec![],
                    min: 1,
                    max: 1,
                    reason: baylee_engine::choice::TargetPrompt::Targets,
                },
                PlayerId::new(0),
            )),
            ..default()
        })
        .init_resource::<crate::input::TrayGlide>()
        .add_systems(Update, pointer);

    // The row as `spawn_stack_entry` builds it: the object it draws, and
    // the marker that says this node is the row rather than a chip.
    let row = app
        .world_mut()
        .spawn((
            crate::hud::HandCardVisual { object: obj(200) },
            crate::hud::StackRowCard,
        ))
        .id();
    // Before the click: the panel draws its halo off `is_selectable`, and
    // a row that answered a click while saying nothing about itself would
    // be a target a player could only find by trying.
    assert!(
        app.world()
            .resource::<crate::Duel>()
            .interaction
            .as_ref()
            .is_some_and(|i| i.is_selectable(obj(200))),
        "the spell on the stack is drawn as an answer the question accepts"
    );
    click(&mut app, row);

    let duel = app.world().resource::<crate::Duel>();
    let picked: Vec<_> = duel
        .interaction
        .as_ref()
        .expect("the question is still standing")
        .selected()
        .collect();
    assert_eq!(picked, vec![obj(200)], "the spell on the stack was chosen");
    assert!(
        duel.interaction
            .as_ref()
            .is_some_and(baylee_client_core::Interaction::can_confirm),
        "and the answer is complete"
    );
}

/// The keyboard's half of the same gap.
///
/// `cursor_grid` was built out of the *board* — the hand and each seat's
/// lanes — and the stack is not on the board, so the card cursor walked
/// past a spell it was being asked to target and never arrived. The stack
/// is the last row because the panel is drawn highest.
#[test]
fn the_card_cursor_reaches_a_spell_on_the_stack() {
    use baylee_client_core::BoardModel;
    use baylee_client_core::board::Openings;
    use baylee_client_core::test_support::{ViewBuilder, printed};

    let view = ViewBuilder::new(2)
        .with_hand(vec![("Ornithopter", 0, 30)])
        .with_stack(vec![printed(200, 0, "Spellseeker", 4)])
        .build();
    let board = BoardModel::from_view(&view, Openings::none(), &[], crate::cardart::registry());
    assert!(
        board.stack.iter().any(|item| item.id == obj(200)),
        "the board model carries the stack"
    );

    let mut duel = crate::Duel {
        board: Some(board),
        ..Default::default()
    };
    let grid = cursor_grid(&duel);
    assert_eq!(
        grid.last().map(Vec::as_slice),
        Some(&[obj(200)][..]),
        "the stack is the topmost row of the grid"
    );

    // From the hand, one step up the grid is the stack, because there is
    // nothing on either board between them.
    duel.hovered = Some(obj(30));
    move_cursor(&mut duel, 1, 0);
    assert_eq!(
        duel.hovered,
        Some(obj(200)),
        "the cursor walks onto the stack"
    );
}

#[test]
fn player_summary_children_target_the_seat_without_moving_the_camera() {
    use bevy::prelude::*;
    let mut app = App::new();
    app.init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::touch::Touched>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Click>>()
        .insert_resource(crate::Duel {
            interaction: Some(baylee_client_core::interaction::Interaction::new(
                Pending::ChooseTargets {
                    player: PlayerId::new(0),
                    options: vec![],
                    player_options: vec![PlayerId::new(1)],
                    min: 1,
                    max: 1,
                    reason: baylee_engine::choice::TargetPrompt::Targets,
                },
                PlayerId::new(0),
            )),
            ..default()
        })
        .init_resource::<crate::input::TrayGlide>()
        .add_systems(Update, pointer);
    let summary = app
        .world_mut()
        .spawn(crate::hud::PlayerTab {
            player: PlayerId::new(1),
        })
        .id();
    let life = app.world_mut().spawn(ChildOf(summary)).id();
    click(&mut app, life);
    assert!(
        app.world()
            .resource::<crate::Duel>()
            .interaction
            .as_ref()
            .unwrap()
            .is_seat_selected(PlayerId::new(1))
    );
    let interaction = app
        .world()
        .resource::<crate::Duel>()
        .interaction
        .as_ref()
        .unwrap();
    assert_eq!(
        interaction.aim(),
        Some(baylee_client_core::interaction::Pick::Seat(PlayerId::new(
            1
        )))
    );
    assert!(
        matches!(interaction.confirm(), Some(PlayerAction::ChooseTargets { players, objects })
        if players == vec![PlayerId::new(1)] && objects.is_empty())
    );
    assert!(app.world().resource::<crate::Duel>().focus.is_none());
    let illegal = app
        .world_mut()
        .spawn(crate::hud::PlayerTab {
            player: PlayerId::new(2),
        })
        .id();
    click(&mut app, illegal);
    assert!(app.world().resource::<crate::Duel>().focus.is_none());
    click(&mut app, life);
    assert_eq!(
        app.world()
            .resource::<crate::Duel>()
            .interaction
            .as_ref()
            .unwrap()
            .pick_count(),
        0
    );
}

#[test]
fn issue_112_timing_and_cost_refusals_are_distinct() {
    use baylee_client_core::i18n::{Phrase, Refusal};
    use baylee_client_core::test_support::ViewBuilder;
    for (active, reason) in [
        (0, Phrase::CardCostsUnavailable),
        (1, Phrase::CardWrongTime),
    ] {
        let mut view = ViewBuilder::new(2)
            .with_hand(vec![("Llanowar Elves", 2, 30)])
            .build();
        view.hand[0].card.index = baylee_cards::decks::by_name("Llanowar Elves").unwrap();
        view.active = PlayerId::new(active);
        let mut duel = crate::Duel {
            view: Some(view),
            interaction: Some(Interaction::new(
                Pending::Priority {
                    player: PlayerId::new(0),
                    legal: Box::default(),
                },
                PlayerId::new(0),
            )),
            ..Default::default()
        };
        assert_eq!(
            activate_card(&mut duel, obj(30)),
            baylee_client_core::touch::Answer::Refused
        );
        assert_eq!(duel.last_error, Some(Refusal::Said(reason)));
        assert!(duel.outbox().is_empty());
    }
}
