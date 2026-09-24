//! A `KeyCode` pressed at the real `keyboard` system, answered out of `Duel::outbox`. `docs/keyboard-map.md` promises every question is answerable without a pointer, and the way that promise breaks is invisible from below: a land the engine had offered, sitting under the cursor, was played by no key at all while every `Interaction` test stayed green. So nothing here is hand-built between the key and the action — the primary key on the card under the cursor, `Y`/`N` and the mulligan pair, and a number typed digit by digit rather than stepped to. Keys that belong to the zone panel instead of to the table are `tray` and `dialog_keys`; the holds `F6`/`F7` set are `menu`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// The whole keyboard path, and not just the decision underneath it.
///
/// `confirming_a_priority_choice_passes` asks the `Interaction` directly,
/// which a live game showed is not enough: a land the engine had offered,
/// sitting under the cursor, was played by no key and no click, and every
/// unit test kept passing. So this one presses a `KeyCode` at the real
/// system and reads the outbox — nothing hand-built in between.
#[test]
fn the_primary_key_plays_the_land_under_the_cursor() {
    use baylee_client_core::prefs::Action;
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::prelude::*;

    let duel = crate::Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(LegalActions {
                    can_pass: true,
                    lands: vec![obj(3)],
                    castable: vec![],
                    mana_abilities: vec![],
                    abilities: vec![],
                    suspendable: vec![],
                }),
            },
            PlayerId::new(0),
        )),
        hovered: Some(obj(3)),
        ..Default::default()
    };

    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .insert_resource(duel)
        .add_systems(Update, keyboard);

    // A precondition, so that a keymap this test cannot see is never the
    // reason it passes: it would pass by pressing nothing at all.
    {
        let prefs = app.world().resource::<crate::prefs::Prefs>();
        let mut probe = ButtonInput::<KeyCode>::default();
        probe.press(KeyCode::Enter);
        assert!(
            crate::keys::Fired::of(&probe, prefs.keymap()).has(Action::Primary),
            "enter is the primary key in the standard map"
        );
    }

    // Once. Playing a land is the one-click case: its whole cost is the
    // land drop and the worst it can go wrong is the wrong land in it.
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Enter);
    }
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        [PlayerAction::PlayLand { card: obj(3) }],
        "the land under the cursor is what the primary key plays"
    );
}

/// Every straight answer to a question, through the keyboard.
///
/// Written to settle entry 36 of `docs/observed-faults.md`, which says a
/// yes/no question is answerable only with the pointer. Half of that is
/// wrong on its face: `Action::AnswerYes` and `AnswerNo` exist,
/// `Keymap::standard` binds them to `Y` and `N`, and
/// `answer_the_question` reads both — so a live run in which `Y` did
/// nothing was stopped by something else, and an entry naming the wrong
/// mechanism sends the fix to the wrong file.
///
/// The mulligan pair is here for the same reason: the entry names it as
/// the same branch, and a claim about two branches wants both pressed.
#[test]
fn a_question_is_answered_from_the_keyboard() {
    use baylee_engine::choice::YesNoPrompt;
    use bevy::prelude::KeyCode;

    let yes_no = |prompt| Pending::YesNo {
        player: PlayerId::new(0),
        prompt,
        source: None,
    };
    assert_eq!(
        pressing(yes_no(YesNoPrompt::Generic), KeyCode::KeyY),
        [PlayerAction::YesNo(true)],
        "Y answers a generic yes/no"
    );
    assert_eq!(
        pressing(yes_no(YesNoPrompt::Generic), KeyCode::KeyN),
        [PlayerAction::YesNo(false)],
        "N answers a generic yes/no"
    );
    // The prompt the live run actually stalled on, in case the shape of
    // the question ever starts deciding whether it can be answered.
    assert_eq!(
        pressing(
            yes_no(YesNoPrompt::PayLifeOrEnterTapped { amount: 2 }),
            KeyCode::KeyY
        ),
        [PlayerAction::YesNo(true)],
        "Y pays the shockland"
    );

    let mulligan = Pending::Mulligan {
        player: PlayerId::new(0),
        taken: 0,
        next_is_free: true,
    };
    assert_eq!(
        pressing(mulligan.clone(), KeyCode::KeyK),
        [PlayerAction::MulliganKeep],
        "K keeps the hand"
    );
    assert_eq!(
        pressing(mulligan, KeyCode::KeyB),
        [PlayerAction::MulliganTake],
        "B takes a mulligan"
    );
}

/// Stepping from 0 to 12 is twelve presses, and X is routinely somebody's
/// whole hand of lands.
#[test]
fn a_number_can_be_typed_rather_than_stepped_to() {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .insert_resource(number_duel(20))
        .add_systems(Update, keyboard);
    let window = app.world_mut().spawn_empty().id();

    let type_digit = |app: &mut App, c: char| {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Digit0,
            logical_key: Key::Character(c.to_string().into()),
            state: bevy::input::ButtonState::Pressed,
            text: Some(c.to_string().into()),
            repeat: false,
            window,
        });
        app.update();
    };

    type_digit(&mut app, '1');
    type_digit(&mut app, '2');
    let number = |app: &App| {
        app.world()
            .resource::<crate::Duel>()
            .interaction
            .as_ref()
            .expect("the choice stands")
            .number()
    };
    assert_eq!(number(&app), 12, "a second digit appends");

    // …and a digit that would leave the range is the whole answer instead,
    // which is what a player means by typing 7 at a maximum of 20.
    type_digit(&mut app, '7');
    assert_eq!(number(&app), 7);

    // Backspace takes one off.
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Backspace,
        logical_key: Key::Backspace,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    assert_eq!(number(&app), 0);
}

/// `⇧E` on a merged card takes the whole of it (#210): pressed at the real
/// keyboard system over twelve Soldiers in a declaration, it declares all
/// twelve, and pressed again over the card they have become it takes all
/// twelve back. `E` alone is one at a time, which is `activate_card`'s.
#[test]
fn shift_e_takes_the_whole_merged_card_and_gives_it_back() {
    use baylee_client_core::prefs::Action;
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::prelude::*;

    let soldiers: Vec<_> = (1..=12)
        .map(|slot| baylee_client_core::test_support::token(slot, 0, "Soldier", 1, 1))
        .collect();
    let ids: Vec<ObjectId> = soldiers.iter().map(|o| o.id).collect();
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2)
        .with_battlefield(0, soldiers)
        .build();
    view.awaiting = Some(view.seat);
    let mut duel = crate::Duel::default();
    duel.receive_view(view);
    duel.receive_choice(Pending::ChooseAttackers {
        player: PlayerId::new(0),
        attackers: ids.clone(),
        defenders: vec![baylee_core::ids::Defender::Player(PlayerId::new(1))],
    });
    crate::rebuild_board(&mut duel);
    duel.hovered = Some(ids[0]);

    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .insert_resource(duel)
        .add_systems(Update, (keyboard, crate::table::track_proposals).chain());

    {
        let prefs = app.world().resource::<crate::prefs::Prefs>();
        let mut probe = ButtonInput::<KeyCode>::default();
        probe.press(KeyCode::ShiftLeft);
        probe.press(KeyCode::KeyE);
        let fired = crate::keys::Fired::of(&probe, prefs.keymap());
        assert!(fired.has(Action::ActivateGroup), "⇧E is the whole card");
        assert!(!fired.has(Action::ActivateCard), "and not also one of it");
    }

    let press = |app: &mut App| {
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release_all();
            keys.clear();
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyE);
        }
        app.update();
        let duel = app.world().resource::<crate::Duel>();
        duel.interaction
            .as_ref()
            .expect("the declaration")
            .declared()
    };
    assert_eq!(press(&mut app), 12, "the whole card was declared");
    assert_eq!(press(&mut app), 0, "and the whole card taken back");
}
