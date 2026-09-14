use baylee_client_core::interaction::Interaction;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

#[test]
fn confirming_a_priority_choice_passes() {
    let i = Interaction::new(
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                lands: vec![obj(1)],
                castable: vec![],
                mana_abilities: vec![],
                abilities: vec![],
                suspendable: vec![],
            }),
        },
        PlayerId::new(0),
    );
    assert_eq!(i.confirm(), Some(PlayerAction::PassPriority));
    // And a click on the land plays it instead of passing.
    assert_eq!(
        i.play_card(obj(1)),
        Some(PlayerAction::PlayLand { card: obj(1) })
    );
}

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
        .add_systems(Update, super::pointer);

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
    let board = BoardModel::from_view(
        &view,
        Openings::none(),
        |_| 12.0,
        crate::cardart::registry(),
    );
    assert!(
        board.stack.iter().any(|item| item.id == obj(200)),
        "the board model carries the stack"
    );

    let mut duel = crate::Duel {
        board: Some(board),
        ..Default::default()
    };
    let grid = super::cursor_grid(&duel);
    assert_eq!(
        grid.last().map(Vec::as_slice),
        Some(&[obj(200)][..]),
        "the stack is the topmost row of the grid"
    );

    // From the hand, one step up the grid is the stack, because there is
    // nothing on either board between them.
    duel.hovered = Some(obj(30));
    super::move_cursor(&mut duel, 1, 0);
    assert_eq!(
        duel.hovered,
        Some(obj(200)),
        "the cursor walks onto the stack"
    );
}

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
        .add_systems(Update, super::keyboard);

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

/// Presses a key at the real system and answers the outbox.
///
/// Shares [`the_primary_key_plays_the_land_under_the_cursor`]'s shape
/// rather than its body: the point of both is that nothing is hand-built
/// between a `KeyCode` and a `PlayerAction`.
fn pressing(pending: Pending, key: bevy::prelude::KeyCode) -> Vec<PlayerAction> {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::prelude::*;

    let duel = crate::Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            pending,
            PlayerId::new(0),
        )),
        ..Default::default()
    };

    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .insert_resource(duel)
        .add_systems(Update, super::keyboard);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    app.world().resource::<crate::Duel>().outbox().to_vec()
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

/// The finger, the hand row and the real `pointer`, with a land to play.
///
/// Everything a tap travels through except the tree that gets rebuilt
/// underneath it — which is the point: the rebuild is done by hand in the
/// tests below, because that is what the client does to itself on every
/// hover change and every arriving view.
fn hand_app() -> bevy::app::App {
    use bevy::picking::events::{Click, Pointer, Press, Release};
    use bevy::prelude::*;

    // Spelled out, because `bevy::prelude` brings a `bevy_ui::Interaction`
    // of its own and shadows the one this client means.
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
        ..Default::default()
    };

    let mut app = App::new();
    app.init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::touch::Touched>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<Pointer<Press>>()
        .add_message::<Pointer<Release>>()
        .add_message::<Pointer<Click>>()
        .insert_resource(duel)
        .add_systems(
            Update,
            (crate::touch::watch_the_finger, super::pointer).chain(),
        );
    let mut window = Window::default();
    window.resolution.set(1728.0, 1052.0);
    app.world_mut().spawn((window, bevy::window::PrimaryWindow));
    app
}

/// One node in the hand row, as `spawn_hand_zone` builds one.
///
/// Both components, because the pair is what the two systems ask for:
/// the wider one is what a press and a click resolve through, the marker
/// is what says this node is the row's and not the stack panel's.
fn row_card(app: &mut bevy::app::App, object: ObjectId) -> bevy::prelude::Entity {
    app.world_mut()
        .spawn((
            crate::hud::HandCardVisual { object },
            crate::hud::HandRowCard,
        ))
        .id()
}

/// Where the pointer is, as the picking backend would report it.
fn pointer_at(app: &mut bevy::app::App) -> bevy::picking::pointer::Location {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::prelude::*;
    use bevy::window::{PrimaryWindow, WindowRef};

    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("the harness made a window");
    let target = WindowRef::Entity(window)
        .normalize(Some(window))
        .expect("a window is a render target");
    bevy::picking::pointer::Location {
        target: NormalizedRenderTarget::Window(target),
        position: Vec2::ZERO,
    }
}

fn finger_down(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
    use bevy::picking::events::{Pointer, Press};
    use bevy::picking::pointer::PointerId;

    let location = pointer_at(app);
    let event = Press {
        button: bevy::picking::pointer::PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
        count: 1,
    };
    app.world_mut()
        .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
}

fn finger_up(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
    use bevy::picking::events::{Pointer, Release};
    use bevy::picking::pointer::PointerId;

    let location = pointer_at(app);
    let event = Release {
        button: bevy::picking::pointer::PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
    };
    app.world_mut()
        .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
}

/// The click bevy raises when the press and the release agree on an
/// entity — the half that goes missing when the tree is rebuilt.
fn the_click(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
    use bevy::picking::events::{Click, Pointer};
    use bevy::picking::pointer::PointerId;

    let location = pointer_at(app);
    let event = Click {
        button: bevy::picking::pointer::PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(entity, 0.0, None, None),
        duration: std::time::Duration::from_millis(10),
        count: 1,
    };
    app.world_mut()
        .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
}

/// A tap the tree ate still plays the card.
///
/// `docs/observed-faults.md` 35. Bevy raises a `Pointer<Click>` only when
/// the press and the release land on the same **entity**, and the hand
/// row is rebuilt on every hover change and on every arriving view — so
/// a finger that is down across one of those comes up on a node born
/// after the press and no click is ever raised. The card sank, came back
/// and played nothing.
#[test]
fn a_tap_that_spans_a_rebuild_still_plays_the_card() {
    let mut app = hand_app();
    let before = row_card(&mut app, obj(3));
    finger_down(&mut app, before);
    app.update();

    // The rebuild: the node the finger went down on is despawned and the
    // same card comes back as a different entity.
    app.world_mut().entity_mut(before).despawn();
    let after = row_card(&mut app, obj(3));
    finger_up(&mut app, after);
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        [PlayerAction::PlayLand { card: obj(3) }],
        "the tap reached the card it was made on"
    );
}

/// The other half of the fault text, and no rebuild in it at all.
///
/// A card's art, its text and its rail are separate pickable children, so
/// a press that drifts across that seam is two entities and bevy raises
/// no click either. It rides on the same lineage walk a click does, which
/// is why one fix covers both.
#[test]
fn a_press_that_drifts_across_one_card_is_still_a_tap() {
    use bevy::prelude::*;

    let mut app = hand_app();
    let card = row_card(&mut app, obj(3));
    let art = app.world_mut().spawn(ChildOf(card)).id();
    let text = app.world_mut().spawn(ChildOf(card)).id();

    finger_down(&mut app, art);
    finger_up(&mut app, text);
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        [PlayerAction::PlayLand { card: obj(3) }],
        "the art and the text are one card"
    );
}

/// A tap the tree *did* hear is sent once, not twice.
///
/// The counter-test the fix above is worth nothing without: the flag is
/// raised by every release over the card the finger is on, including the
/// ordinary ones, and is meant to be taken down again by the click that
/// answers them. Read before the clicks instead of after, this plays the
/// land and then plays it again.
#[test]
fn a_tap_the_tree_heard_is_sent_once() {
    let mut app = hand_app();
    let card = row_card(&mut app, obj(3));
    finger_down(&mut app, card);
    app.update();

    // Press and release on the same entity, so bevy raises the click too
    // — all three on the frame the release lands, as they arrive live.
    finger_up(&mut app, card);
    the_click(&mut app, card);
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        [PlayerAction::PlayLand { card: obj(3) }],
        "one tap is one land"
    );
}

/// A press dragged off its card and let go over another one asks nothing.
///
/// The second counter-test: a flag raised on every release, rather than
/// only on a release over the card the finger went down on, would turn
/// every dragged-off press into a tap on whatever it started on.
#[test]
fn a_press_let_go_over_another_card_sends_nothing() {
    let mut app = hand_app();
    let land = row_card(&mut app, obj(3));
    let other = row_card(&mut app, obj(5));
    finger_down(&mut app, land);
    app.update();
    finger_up(&mut app, other);
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        [],
        "a press that moved on is not a tap"
    );
}

/// Builds the app the two menu-button tests share: the real `pointer`
/// system, and a click helper that goes through the real message.
fn menu_app(duel: crate::Duel) -> (bevy::app::App, bevy::prelude::Entity, bevy::prelude::Entity) {
    use crate::hud::MenuAction;
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::touch::Touched>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Click>>()
        .insert_resource(duel)
        .add_systems(Update, super::pointer);
    let draw = app
        .world_mut()
        .spawn(crate::hud::MenuButton {
            action: MenuAction::OfferDraw,
        })
        .id();
    let concede = app
        .world_mut()
        .spawn(crate::hud::MenuButton {
            action: MenuAction::Concede,
        })
        .id();
    (app, draw, concede)
}

/// One click on one entity, as the picking backend would report it.
fn click(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::events::{Click, Pointer};
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::prelude::*;
    use bevy::window::{PrimaryWindow, WindowRef};

    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .unwrap_or_else(|_| {
            app.world_mut()
                .spawn((Window::default(), PrimaryWindow))
                .id()
        });
    let camera = app.world_mut().spawn_empty().id();
    let target = WindowRef::Entity(window)
        .normalize(Some(window))
        .expect("a window is a render target");
    let location = Location {
        target: NormalizedRenderTarget::Window(target),
        position: Vec2::ZERO,
    };
    let event = Click {
        button: bevy::picking::pointer::PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
        duration: std::time::Duration::from_millis(10),
        count: 1,
    };
    app.world_mut()
        .write_message(Pointer::new(PointerId::Mouse, location, event, entity));
    app.update();
}

/// Playing the card under the pointer must take its preview with it.
///
/// The hand zone is rebuilt whole on every board change, so the node the
/// pointer was over is *despawned* — and Bevy fires no `Out` for an
/// entity that no longer exists. The card preview therefore stayed open
/// over the middle of the table until the player happened to hover
/// something else, which is how a screenshot of a live game found it.
///
/// The second half is the part a bare "does this object still exist"
/// check would fail: a land goes on playing under the same `ObjectId`,
/// now as a permanent, so the hover is only stale because it came from
/// the *hand*.
#[test]
fn a_hand_card_that_is_played_takes_its_hover_with_it() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
        .add_message::<bevy::window::CursorMoved>()
        .insert_resource(crate::Duel::default())
        .add_systems(Update, super::pointer_hover);

    let card = app
        .world_mut()
        .spawn(crate::hud::HandCardVisual { object: obj(3) })
        .id();
    hover(&mut app, card);
    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        Some(obj(3)),
        "the pointer over a hand card is a hover"
    );

    // The land is played: the hand zone is rebuilt without it, and the
    // same object arrives on the table. No `Out` is fired, and the
    // pointer does not move.
    app.world_mut().entity_mut(card).despawn();
    app.world_mut().spawn(crate::table::CardVisual {
        object: obj(3),
        count: 1,
    });
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        None,
        "a hand card that left the hand is not still hovered"
    );
}

/// The cure must not bring back the disease.
///
/// `Duel::hovered` has four writers and `pointer_hover` is only one of
/// them: the keyboard cursor writes it too, and the cursor walking off a
/// permanent and onto a hand card is exactly that. Held against the
/// *pointer's* last source, that write would be checked against the table,
/// not found there and cleared on the next frame — the stall of "the
/// pointer only speaks when it moves" back through a different door. So a
/// hover this system did not write is nobody's kind in particular.
#[test]
fn a_hover_this_system_did_not_write_is_left_alone() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
        .add_message::<bevy::window::CursorMoved>()
        .insert_resource(crate::Duel::default())
        .add_systems(Update, super::pointer_hover);

    let permanent = app
        .world_mut()
        .spawn(crate::table::CardVisual {
            object: obj(7),
            count: 1,
        })
        .id();
    app.world_mut()
        .spawn(crate::hud::HandCardVisual { object: obj(3) });
    hover(&mut app, permanent);
    assert_eq!(app.world().resource::<crate::Duel>().hovered, Some(obj(7)));

    // What `move_cursor` does when the keyboard walks onto the hand: it
    // writes the hover directly, and the pointer has not moved.
    app.world_mut().resource_mut::<crate::Duel>().hovered = Some(obj(3));
    app.update();
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        Some(obj(3)),
        "the keyboard cursor survives a pointer that is resting elsewhere"
    );
}

/// The other half of `a_hand_card_that_is_played_takes_its_hover_with_it`.
///
/// The same event — the card under the hover is played, its hand node is
/// despawned and a permanent appears under the same `ObjectId` — and the
/// answer is the opposite one, because the two hovers are valid for
/// different reasons. The pointer's is over an entity that no longer
/// exists, so it goes. The keyboard's is a position in `cursor_grid`,
/// the card is still in that grid one row down, and taking it away would
/// send the player's next arrow key back to the start of their hand.
///
/// Written as a test rather than left to the comment because the union in
/// the `Elsewhere` arm reads like the permissive fallback of the other
/// two, and the next person to tighten it will have this fail.
#[test]
fn the_keyboard_cursor_follows_a_card_it_played_onto_the_table() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
        .add_message::<bevy::window::CursorMoved>()
        .insert_resource(crate::Duel::default())
        .add_systems(Update, super::pointer_hover);

    // A land in hand, with the keyboard cursor on it: written straight to
    // the resource, which is what `move_cursor` does.
    let in_hand = app
        .world_mut()
        .spawn(crate::hud::HandCardVisual { object: obj(5) })
        .id();
    app.world_mut().resource_mut::<crate::Duel>().hovered = Some(obj(5));
    app.update();

    // It is played. The hand zone is rebuilt without it and the same
    // object is now a permanent.
    app.world_mut().entity_mut(in_hand).despawn();
    app.world_mut().spawn(crate::table::CardVisual {
        object: obj(5),
        count: 1,
    });
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        Some(obj(5)),
        "the cursor should follow the card it just played, not reset"
    );
}

/// An app with `pointer_hover` and one permanent on the table, seen from
/// a seat that also has a graveyard to lose it into.
fn hover_app(view: baylee_view::PlayerView) -> (bevy::app::App, bevy::prelude::Entity) {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_message::<bevy::picking::events::Pointer<bevy::picking::events::Over>>()
        .add_message::<bevy::picking::events::Pointer<bevy::picking::events::Out>>()
        .add_message::<bevy::window::CursorMoved>()
        .insert_resource(crate::Duel {
            view: Some(view),
            ..crate::Duel::default()
        })
        .add_systems(Update, super::pointer_hover);
    let card = app
        .world_mut()
        .spawn(crate::table::CardVisual {
            object: obj(9),
            count: 1,
        })
        .id();
    (app, card)
}

/// A fetchland cracked under the pointer, which is the everyday way into
/// this and the way it was found.
///
/// The card is sacrificed, glides to the graveyard and becomes the top of
/// it — keeping the very entity it had on the battlefield, because
/// `SceneIndex::cards` reuses one entity per object. So it is still drawn
/// and still a `CardVisual`, the pointer has not moved so no `Out` is
/// fired, and the hover crossed the table with it. `the_click` answers a
/// hover before anything else, so the `Enter` meant for the search the
/// fetchland had just opened opened the graveyard instead.
#[test]
fn a_card_that_leaves_the_battlefield_leaves_the_pointer_behind() {
    use baylee_client_core::test_support::{ViewBuilder, printed};

    let on_the_field = ViewBuilder::new(2)
        .with_battlefield(0, [printed(9, 0, "Marsh Flats", 4)])
        .build();
    let (mut app, card) = hover_app(on_the_field);
    hover(&mut app, card);
    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        Some(obj(9)),
        "the pointer over a permanent is a hover"
    );

    // Cracked. The same entity is now the top of the graveyard, and
    // nothing else about the frame has changed.
    app.world_mut().resource_mut::<crate::Duel>().view = Some(
        ViewBuilder::new(2)
            .with_graveyard(0, vec![printed(9, 0, "Marsh Flats", 4)])
            .build(),
    );
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        None,
        "the hover followed the card into the graveyard"
    );
}

/// The cure must not take the ordinary hover away.
///
/// Cards on this table move constantly — a lane repacks, a permanent taps,
/// a hovered card lifts — and the pointer is meant to keep its card
/// through all of it. Only a card that has changed *zone* has left the
/// place the pointer is making a claim about.
#[test]
fn a_permanent_that_only_moves_keeps_its_hover() {
    use baylee_client_core::test_support::{ViewBuilder, printed, token};

    let alone = ViewBuilder::new(2)
        .with_battlefield(0, [printed(9, 0, "Birds of Paradise", 4)])
        .build();
    let (mut app, card) = hover_app(alone);
    hover(&mut app, card);
    assert_eq!(app.world().resource::<crate::Duel>().hovered, Some(obj(9)));

    // A second creature arrives, the lane repacks, and the hovered card
    // is drawn somewhere else entirely. It is still on the battlefield.
    app.world_mut().resource_mut::<crate::Duel>().view = Some(
        ViewBuilder::new(2)
            .with_battlefield(
                0,
                [
                    printed(9, 0, "Birds of Paradise", 4),
                    token(11, 0, "Saproling", 1, 1),
                ],
            )
            .build(),
    );
    app.update();
    app.update();

    assert_eq!(
        app.world().resource::<crate::Duel>().hovered,
        Some(obj(9)),
        "a repacked lane is not a card leaving the pointer"
    );
}

/// One pointer entering one entity, as the picking backend would report
/// it. The cursor move is what opens the system's grace window: a still
/// pointer is deliberately silent.
fn hover(app: &mut bevy::app::App, entity: bevy::prelude::Entity) {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::events::{Over, Pointer};
    use bevy::picking::pointer::{Location, PointerId};
    use bevy::prelude::*;
    use bevy::window::{PrimaryWindow, WindowRef};

    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .unwrap_or_else(|_| {
            app.world_mut()
                .spawn((Window::default(), PrimaryWindow))
                .id()
        });
    let camera = app.world_mut().spawn_empty().id();
    let target = WindowRef::Entity(window)
        .normalize(Some(window))
        .expect("a window is a render target");
    let location = Location {
        target: NormalizedRenderTarget::Window(target),
        position: Vec2::ZERO,
    };
    app.world_mut().write_message(bevy::window::CursorMoved {
        window,
        position: Vec2::ZERO,
        delta: None,
    });
    app.world_mut().write_message(Pointer::new(
        PointerId::Mouse,
        location,
        Over {
            hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
        },
        entity,
    ));
    app.update();
}

/// The engine refuses a draw offer outside the offerer's own priority, so
/// the button used to be a live button whose usual answer was an error.
#[test]
fn a_draw_is_only_offered_from_this_seats_own_priority() {
    use bevy::prelude::*;

    // A choice that is not priority: the offer is not sent.
    let (mut app, draw, _) = menu_app(crate::Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            Pending::YesNo {
                player: PlayerId::new(0),
                prompt: baylee_engine::choice::YesNoPrompt::Generic,
                source: None,
            },
            PlayerId::new(0),
        )),
        ..Default::default()
    });
    click(&mut app, draw);
    assert!(
        app.world().resource::<crate::Duel>().outbox().is_empty(),
        "a draw was offered without priority, which the engine refuses"
    );

    // And with priority it goes.
    let (mut app, draw, _) = menu_app(crate::Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(LegalActions {
                    can_pass: true,
                    lands: vec![],
                    castable: vec![],
                    mana_abilities: vec![],
                    abilities: vec![],
                    suspendable: vec![],
                }),
            },
            PlayerId::new(0),
        )),
        ..Default::default()
    });
    click(&mut app, draw);
    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        [PlayerAction::OfferDraw]
    );
}

/// A view button is the one control on the dialog that writes to the
/// settings store rather than to the `Browser`.
///
/// Two halves, and the second is the one worth a test. The first is that
/// the click lands at all — three new buttons that change nothing is the
/// defect the sort key already shipped once. The second is that it writes
/// **only** the view: the sheet's remembered rectangle lives in the same
/// resource, and a handler that wrote the whole store back would park a
/// sheet nobody had moved.
#[test]
fn a_view_button_writes_the_view_and_nothing_else() {
    use crate::settings::ClientSettings;
    use baylee_client_core::browser::ViewMode;

    let (mut app, _, _) = menu_app(crate::Duel::default());
    let grid = app
        .world_mut()
        .spawn(crate::hud::TrayView {
            mode: ViewMode::Grid,
        })
        .id();
    let detailed = app
        .world_mut()
        .spawn(crate::hud::TrayView {
            mode: ViewMode::Detailed,
        })
        .id();

    assert_eq!(
        app.world().resource::<ClientSettings>().zone_view,
        ViewMode::Detailed,
        "the list is what a player who has chosen nothing gets"
    );

    click(&mut app, grid);
    assert_eq!(
        app.world().resource::<ClientSettings>().zone_view,
        ViewMode::Grid,
        "the grid button did not reach the setting the panel is drawn from"
    );
    assert!(
        app.world()
            .resource::<ClientSettings>()
            .zone_browser
            .is_none(),
        "changing the view wrote a place nobody chose"
    );

    click(&mut app, detailed);
    assert_eq!(
        app.world().resource::<ClientSettings>().zone_view,
        ViewMode::Detailed,
        "the view is a choice, not a ratchet"
    );
}

/// One misclick used to end a ranked game.
#[test]
fn conceding_takes_two_presses_and_anything_else_forgets_the_first() {
    let (mut app, draw, concede) = menu_app(crate::Duel::default());

    click(&mut app, concede);
    assert!(
        app.world().resource::<crate::Duel>().outbox().is_empty(),
        "one press conceded the game"
    );
    assert!(app.world().resource::<crate::Duel>().concede_armed);

    // Anything else in between and the first press is forgotten.
    click(&mut app, draw);
    assert!(!app.world().resource::<crate::Duel>().concede_armed);
    click(&mut app, concede);
    assert!(app.world().resource::<crate::Duel>().outbox().is_empty());

    // Twice in a row, and it goes.
    click(&mut app, concede);
    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        [PlayerAction::Concede]
    );
}

/// A number choice with a range, ready to be typed at.
fn number_duel(max: u32) -> crate::Duel {
    crate::Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            Pending::ChooseNumber {
                player: PlayerId::new(0),
                min: 0,
                max,
            },
            PlayerId::new(0),
        )),
        ..Default::default()
    }
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
        .add_systems(Update, super::keyboard);
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

/// The graveyard is searchable, and the search term stays out of the game.
///
/// "Sortierbar, durchsuchbar, scrollbar" — the first and the last were
/// there and the middle one was not: `Browser::set_filter` was written
/// and no key or click ever reached it. What is pinned here is both
/// halves of the bargain: the box holds the keyboard from the moment the
/// sheet opens, and it lets go when the player says so, after which the
/// panel can stand open for a whole turn with the letters belonging to
/// the game again.
///
/// The first half is the owner's bug of 14.09., and it is pinned with the
/// word they actually typed. Fifteen of the twenty-six letters are bound,
/// so a German search term is a handful of game actions: `T` latched the
/// text view on and persisted it — every card in the duel drawn as its own
/// rules text — and `K`/`B`, `Y`/`N` would have answered a mulligan or a
/// yes/no question outright.
#[test]
fn a_search_term_reaches_the_box_and_never_the_game() {
    use bevy::prelude::*;

    let (mut app, window) = a_zone_dialog_that_has_just_opened();
    assert!(
        app.world().resource::<crate::Duel>().browser.is_typing(),
        "the sheet opened and left the keyboard with the table"
    );

    // The owner's own search term, typed into the panel they had just
    // opened. `S`, `T`, `E` and `F` are all bound; `T` is `ToggleTextView`
    // and the one that stayed, because it is written to disk.
    for (code, c) in [
        (KeyCode::KeyS, 's'),
        (KeyCode::KeyT, 't'),
        (KeyCode::KeyU, 'u'),
        (KeyCode::KeyR, 'r'),
        (KeyCode::KeyM, 'm'),
        (KeyCode::KeyT, 't'),
        (KeyCode::KeyI, 'i'),
        (KeyCode::KeyE, 'e'),
        (KeyCode::KeyF, 'f'),
    ] {
        type_letter(&mut app, window, code, c);
    }
    assert_eq!(
        filter_reads(&app),
        "sturmtief",
        "the search term missed the box"
    );
    assert!(panel_stands(&app), "a letter in the term closed the panel");
    assert!(
        !app.world()
            .resource::<crate::settings::ClientSettings>()
            .prefer_text_view,
        "the search term latched the text view on"
    );
    assert!(
        app.world().resource::<crate::Duel>().outbox().is_empty(),
        "the search term sent something to the engine"
    );
}

/// The other half of the same bargain: the box lets go on request, and
/// then the sheet can stand open for a whole turn with the letters
/// belonging to the game again.
#[test]
fn a_released_filter_box_hands_the_letters_back() {
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::prelude::*;

    let (mut app, window) = a_zone_dialog_that_has_just_opened();
    type_letter(&mut app, window, KeyCode::KeyM, 'm');
    type_letter(&mut app, window, KeyCode::KeyO, 'o');
    assert_eq!(filter_reads(&app), "mo");

    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Backspace,
        logical_key: Key::Backspace,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    assert_eq!(filter_reads(&app), "m", "backspace did not reach the box");

    // `G` is `ToggleBrowser`, so the claim is not merely that the filter
    // stopped growing — a key that went nowhere at all would satisfy
    // that. The action has to have *fired*.
    app.world_mut()
        .resource_mut::<crate::Duel>()
        .browser
        .stop_typing();
    let was = panel_stands(&app);
    type_letter(&mut app, window, KeyCode::KeyG, 'g');
    assert_eq!(filter_reads(&app), "m", "the box typed after letting go");
    assert_ne!(panel_stands(&app), was, "a released box ate a bound key");
}

/// An app holding the key path, with the zone dialog one frame past
/// opening.
///
/// That frame matters and is why it is spent here: in the client it is
/// the frame `G` was pressed on, so it is also the frame whose keystroke
/// the box must not take. Letters typed after it are the player's *next*
/// ones, which is what a real keyboard produces.
fn a_zone_dialog_that_has_just_opened() -> (bevy::prelude::App, bevy::prelude::Entity) {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::prelude::*;

    let mut app = App::new();
    let mut duel = crate::Duel::default();
    duel.browser.open();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .init_resource::<Keystrokes>()
        .insert_resource(duel)
        .add_systems(PreUpdate, deliver_keystrokes)
        .add_systems(
            Update,
            (
                super::browser_takes_the_keyboard.before(super::keyboard),
                super::keyboard,
            ),
        );
    let window = app.world_mut().spawn_empty().id();
    app.update();
    (app, window)
}

/// Keystrokes waiting to be written *inside* a frame.
///
/// A message written before `App::update` is not the same message a
/// keyboard writes, and the difference is exactly the one under test.
/// `Messages::update` runs in `First`, so a write from outside the
/// schedule is already one swap old when `Update` sees it and is dropped
/// at the start of the next frame — it lives one frame, not two. A real
/// keystroke is written by `bevy_input` in `PreUpdate`, *after* that swap,
/// so it is still readable on the following frame. That following frame is
/// the frame the filter box takes the keyboard on, which is the whole
/// reason `keyboard` clears its reader there.
///
/// Writing from outside hid that: with the guard commented out the test
/// still passed, because the character the box would have eaten had
/// already expired.
#[derive(bevy::prelude::Resource, Default)]
struct Keystrokes(Vec<bevy::input::keyboard::KeyboardInput>);

/// `bevy_input`'s half, in the one place it matters.
fn deliver_keystrokes(
    mut queued: bevy::prelude::ResMut<Keystrokes>,
    mut out: bevy::prelude::MessageWriter<bevy::input::keyboard::KeyboardInput>,
) {
    for key in queued.0.drain(..) {
        out.write(key);
    }
}

/// One letter, pressed the way a real keyboard presses it.
///
/// A letter is two things at once — a character for a text box and a
/// bound action for the game — and a real press sends both, so this does
/// too. Nothing in these apps clears `ButtonInput` between frames (that
/// is `bevy_input`'s own system, and they have none), so a press has to
/// be released by hand or every later frame sees it held.
fn type_letter(
    app: &mut bevy::prelude::App,
    window: bevy::prelude::Entity,
    code: bevy::prelude::KeyCode,
    c: char,
) {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::prelude::*;

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(code);
    app.world_mut()
        .resource_mut::<Keystrokes>()
        .0
        .push(KeyboardInput {
            key_code: code,
            logical_key: Key::Character(c.to_string().into()),
            state: bevy::input::ButtonState::Pressed,
            text: Some(c.to_string().into()),
            repeat: false,
            window,
        });
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
}

fn filter_reads(app: &bevy::prelude::App) -> String {
    app.world()
        .resource::<crate::Duel>()
        .browser
        .filter()
        .to_string()
}

fn panel_stands(app: &bevy::prelude::App) -> bool {
    app.world().resource::<crate::Duel>().browser.is_open()
}

/// The browser had a pointer route and no keyboard one, which is exactly
/// the promise `docs/keyboard-map.md` makes and the reason the action was
/// added rather than the chip being the only way in.
///
/// It is also the one test that walks the whole `G` path with both systems
/// registered, which is what makes it the place the `typed.clear()` guard
/// is held: the frame `G` opens the sheet on writes a `KeyboardInput` that
/// nothing reads, and messages live two frames, so without the guard that
/// `g` is waiting in the queue when the box takes the keyboard a frame
/// later. The panel would open with `g` already typed into it.
///
/// The second half of the old test — `G` again shuts it — was true and is
/// no longer, which is the *point* of the change rather than a regression:
/// a box with the keyboard is a box a bound letter cannot reach past. The
/// way out is `Esc`, and then the latch is a latch again.
#[test]
fn the_browser_key_opens_the_tray_and_the_box_then_holds_the_letters() {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .init_resource::<crate::Duel>()
        .init_resource::<Keystrokes>()
        .add_systems(PreUpdate, deliver_keystrokes)
        .add_systems(
            Update,
            (
                super::browser_takes_the_keyboard.before(super::keyboard),
                super::keyboard,
            ),
        );
    let window = app.world_mut().spawn_empty().id();

    // `reset_all` and not `clear`: a key that is still held is not pressed
    // again, and the second press would fire nothing at all.
    let press = |app: &mut App, key: KeyCode| {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(key);
        app.update();
    };

    assert!(!panel_stands(&app));
    // A real press of `G`, character and all — because the character is
    // the thing that must not be typed.
    type_letter(&mut app, window, KeyCode::KeyG, 'g');
    assert!(panel_stands(&app), "the browser key did not open the tray");
    assert!(
        !app.world().resource::<crate::Duel>().browser.is_typing(),
        "the box takes the keyboard a frame later, not on the frame the \
         sheet opens: that frame's keystroke belongs to the game"
    );

    // The frame after, which in the client is simply the next one.
    app.update();
    assert!(
        app.world().resource::<crate::Duel>().browser.is_typing(),
        "the sheet opened and nothing gave the filter box the keyboard"
    );
    assert_eq!(
        filter_reads(&app),
        "",
        "the keystroke that opened the panel was typed into it"
    );

    // Now the same key is a letter. The latch is out of reach.
    type_letter(&mut app, window, KeyCode::KeyG, 'g');
    assert_eq!(filter_reads(&app), "g");
    assert!(
        panel_stands(&app),
        "a letter typed into the box shut the panel"
    );

    // `Esc` twice: the first empties the box, the second hands the
    // keyboard back. Two presses because a player who has typed a term
    // means the term, not the panel.
    press(&mut app, KeyCode::Escape);
    assert_eq!(filter_reads(&app), "");
    assert!(app.world().resource::<crate::Duel>().browser.is_typing());
    press(&mut app, KeyCode::Escape);
    assert!(!app.world().resource::<crate::Duel>().browser.is_typing());
    assert!(panel_stands(&app), "letting go of the box shut the panel");

    // And with the letters back at the table it is a latch again.
    press(&mut app, KeyCode::KeyG);
    assert!(
        !panel_stands(&app),
        "it is a latch, so the same key shuts it"
    );
}

/// The search box answers the keys a text field answers.
///
/// The owner named the lobby's boxes as the thing this one should be, and
/// this is the half a player presses: a caret that moves by character and
/// by word, Home and End, shift extending a selection, Delete beside
/// Backspace, and ⌘A. The box was a `String` with characters pushed onto
/// the end of it, so every one of these did nothing — and ⌘A typed an
/// "a", which is the one that also *corrupts* the search.
#[test]
fn the_search_box_answers_the_keys_a_text_field_answers() {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::prelude::*;

    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .init_resource::<crate::Duel>()
        .init_resource::<Keystrokes>()
        .add_systems(PreUpdate, deliver_keystrokes)
        .add_systems(Update, super::keyboard);
    let window = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<crate::Duel>().browser.open();
    app.world_mut()
        .resource_mut::<crate::Duel>()
        .browser
        .start_typing();
    app.update();

    // One key, with whatever modifiers are named held down for it.
    let chord = |app: &mut App, code: KeyCode, key: Key, mods: &[KeyCode]| {
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            for m in mods {
                keys.press(*m);
            }
            keys.press(code);
        }
        app.world_mut()
            .resource_mut::<Keystrokes>()
            .0
            .push(KeyboardInput {
                key_code: code,
                logical_key: key,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            });
        app.update();
    };
    let caret = |app: &App| {
        app.world()
            .resource::<crate::Duel>()
            .browser
            .filter_field()
            .cursor()
    };

    for c in "Wald".chars() {
        type_letter(&mut app, window, KeyCode::KeyW, c);
    }
    assert_eq!(filter_reads(&app), "Wald");
    assert_eq!(caret(&app), 4);

    // Home, then one character right, then a letter typed *inside* the
    // word — the whole thing a caret is for.
    chord(&mut app, KeyCode::Home, Key::Home, &[]);
    assert_eq!(caret(&app), 0);
    chord(&mut app, KeyCode::ArrowRight, Key::ArrowRight, &[]);
    type_letter(&mut app, window, KeyCode::KeyU, 'u');
    assert_eq!(
        filter_reads(&app),
        "Wuald",
        "the caret was not where it said"
    );

    // ⇧End selects to the end, and typing replaces what is selected.
    chord(&mut app, KeyCode::End, Key::End, &[KeyCode::ShiftLeft]);
    assert_eq!(
        app.world()
            .resource::<crate::Duel>()
            .browser
            .filter_field()
            .selection(),
        Some(2..5),
        "shift did not extend a selection"
    );
    type_letter(&mut app, window, KeyCode::KeyO, 'o');
    assert_eq!(filter_reads(&app), "Wuo");

    // Delete forwards from the start, which Backspace cannot do.
    chord(&mut app, KeyCode::Home, Key::Home, &[]);
    chord(&mut app, KeyCode::Delete, Key::Delete, &[]);
    assert_eq!(filter_reads(&app), "uo");

    // And ⌘A selects the box instead of typing an "a" into it.
    chord(
        &mut app,
        KeyCode::KeyA,
        Key::Character("a".into()),
        &[KeyCode::SuperLeft],
    );
    assert_eq!(
        filter_reads(&app),
        "uo",
        "the command chord typed its own letter into the search"
    );
    chord(&mut app, KeyCode::Backspace, Key::Backspace, &[]);
    assert_eq!(
        filter_reads(&app),
        "",
        "select-all and one press empties it"
    );
}

/// A hold is the one statement a seat makes while it is *not* being asked,
/// which is also what makes it dangerous: the prompt bar is empty because
/// the seat is not being asked, and an empty prompt bar is what an idle
/// turn looks like too. So the key that sets a hold has to be the key that
/// takes it back, and it has to work from a view alone.
#[test]
fn the_hold_keys_stop_the_questions_and_take_it_back() {
    use baylee_engine::choice::PriorityHold;
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::prelude::*;

    use crate::host::{DuelHost, HostMessage, LocalHost};
    let mut host = LocalHost::new(
        &crate::host::tests::duel_preset(),
        PlayerId::new(0),
        &["You", "AI"],
    )
    .expect("host");
    // A real view rather than a hand-built one: `hold_action` reads the
    // turn number and the stack depth off it, and a view assembled by the
    // test would only ever agree with the test.
    let view = host
        .poll()
        .into_iter()
        .find_map(|m| match m {
            HostMessage::View(v) => Some(*v),
            _ => None,
        })
        .expect("a view");
    let turn = view.turn;
    let depth = u16::try_from(view.stack.len()).expect("an opening stack fits");

    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .insert_resource(crate::Duel {
            view: Some(view),
            ..Default::default()
        })
        .add_systems(Update, super::keyboard);

    // `reset_all` and not `clear`: a key still held is not pressed again.
    let press = |app: &mut App, key: KeyCode| {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(key);
        app.update();
    };
    let held = |app: &mut App, held: bool| {
        app.world_mut()
            .resource_mut::<crate::Duel>()
            .view
            .as_mut()
            .expect("the view is still there")
            .priority_held = held;
    };

    press(&mut app, KeyCode::F6);
    press(&mut app, KeyCode::F7);
    assert_eq!(
        app.world().resource::<crate::Duel>().outbox(),
        [
            PlayerAction::SetPriorityHold(PriorityHold::UntilStackEmpty { depth }),
            PlayerAction::SetPriorityHold(PriorityHold::UntilEndOfTurn { turn }),
        ],
        "the two hold keys must say two different things"
    );

    // The engine took it; the view says so. Now either key is the way out,
    // because a player who has stopped being asked should not have to
    // remember which one they pressed.
    held(&mut app, true);
    press(&mut app, KeyCode::F7);
    held(&mut app, true);
    press(&mut app, KeyCode::F6);
    assert_eq!(
        app.world().resource::<crate::Duel>().outbox()[2..],
        [
            PlayerAction::SetPriorityHold(PriorityHold::Always),
            PlayerAction::SetPriorityHold(PriorityHold::Always),
        ],
        "a running hold must be cancelled by either key, never replaced"
    );
}

/// The same way out, for a player who never finds a function key.
#[test]
fn the_prompt_bar_can_take_a_hold_back_too() {
    use baylee_engine::choice::PriorityHold;

    use crate::host::{DuelHost, HostMessage, LocalHost};
    use crate::hud::MenuAction;
    let mut host = LocalHost::new(
        &crate::host::tests::duel_preset(),
        PlayerId::new(0),
        &["You", "AI"],
    )
    .expect("host");
    let mut view = host
        .poll()
        .into_iter()
        .find_map(|m| match m {
            HostMessage::View(v) => Some(*v),
            _ => None,
        })
        .expect("a view");
    view.priority_held = true;
    let mut duel = crate::Duel {
        view: Some(view),
        ..Default::default()
    };

    super::menu_click(&mut duel, MenuAction::ReleaseHold, false);
    assert_eq!(
        duel.outbox(),
        [PlayerAction::SetPriorityHold(PriorityHold::Always)]
    );

    // And with nothing to release it sends nothing, rather than setting a
    // hold from the button that exists to cancel one.
    duel.view.as_mut().expect("the view").priority_held = false;
    let mut fresh = crate::Duel {
        view: duel.view.clone(),
        ..Default::default()
    };
    super::menu_click(&mut fresh, MenuAction::ReleaseHold, false);
    assert!(fresh.outbox().is_empty());
}

/// And it can ask for one, against the stack standing over it.
///
/// The twin of the test above, and the half that was missing: `ledge.rs`
/// draws a button carrying [`MenuAction::HoldForStack`] and nothing said
/// the press reached [`Duel::hold_action`]. It cannot be read off a
/// running game either — a hold and a pass both leave the stack resolved
/// and this seat asked again on an empty one — so the outbox is the only
/// place the two differ at all.
///
/// Three states, because the predicate has three answers and two of them
/// are refusals. A hold over a stack of one is `UntilStackEmpty { depth:
/// 1 }`; an empty stack would be `depth: 0`, a hold that is over before it
/// begins; and a hold already running would send `Always`, which cancels
/// the very thing the label promises to set. That last one is what the
/// predicate is for — the other two could have been a greyed-out button.
///
/// [`Duel::hold_action`]: crate::Duel::hold_action
#[test]
fn the_prompt_bar_can_ask_for_a_hold_as_well() {
    use baylee_engine::choice::PriorityHold;

    use crate::host::{DuelHost, HostMessage, LocalHost};
    use crate::hud::MenuAction;
    let mut host = LocalHost::new(
        &crate::host::tests::duel_preset(),
        PlayerId::new(0),
        &["You", "AI"],
    )
    .expect("host");
    let view = host
        .poll()
        .into_iter()
        .find_map(|m| match m {
            HostMessage::View(v) => Some(*v),
            _ => None,
        })
        .expect("a view");

    let seated = |on_stack: bool, held: bool| {
        let mut view = view.clone();
        view.stack = if on_stack {
            vec![baylee_client_core::test_support::token(9, 1, "Shock", 0, 0)]
        } else {
            Vec::new()
        };
        view.priority_held = held;
        crate::Duel {
            view: Some(view),
            ..Default::default()
        }
    };

    let mut duel = seated(true, false);
    super::menu_click(&mut duel, MenuAction::HoldForStack, false);
    assert_eq!(
        duel.outbox(),
        [PlayerAction::SetPriorityHold(
            PriorityHold::UntilStackEmpty { depth: 1 }
        )]
    );

    // Nothing on the stack: the same press would ask for a hold that is
    // already over, so it asks for nothing at all.
    let mut nothing = seated(false, false);
    super::menu_click(&mut nothing, MenuAction::HoldForStack, false);
    assert!(nothing.outbox().is_empty());

    // And with one already running it sends nothing either, rather than
    // the `Always` that would end it — cancelling is `ReleaseHold`'s job.
    let mut running = seated(true, true);
    super::menu_click(&mut running, MenuAction::HoldForStack, false);
    assert!(running.outbox().is_empty());
}

/// A duel driven to seat 0's first main phase, out of a real `LocalHost`.
///
/// Every one of the arming tests needs a `LegalActions` that actually
/// offers something, and a hand-built one offers whatever the test wanted
/// it to. This plays the opening the way the client does — keep the hand,
/// pass until the engine offers a land — and stops at the window where a
/// player has a decision to make.
fn duel_in_main_phase() -> (crate::Duel, crate::host::LocalHost) {
    use crate::host::{DuelHost, HostMessage, LocalHost};
    use baylee_engine::choice::Pending;

    let seat = PlayerId::new(0);
    let mut host = LocalHost::new(&crate::host::tests::duel_preset(), seat, &["You", "AI"])
        .expect("the preset makes a game");
    let mut duel = crate::Duel::default();
    for _ in 0..64 {
        for message in host.poll() {
            match message {
                HostMessage::Static(s) => duel.statics = Some(*s),
                HostMessage::View(v) => duel.view = Some(*v),
                HostMessage::Choice(p) => {
                    duel.interaction = Some(Interaction::new(*p, seat));
                }
                HostMessage::Failed(why) => panic!("the host refused: {why}"),
            }
        }
        match duel.interaction.as_ref().map(Interaction::pending) {
            Some(Pending::Mulligan { .. }) => host.submit(PlayerAction::MulliganKeep),
            Some(Pending::Priority { legal, .. }) if !legal.lands.is_empty() => {
                return (duel, host);
            }
            Some(Pending::Priority { .. }) => host.submit(PlayerAction::PassPriority),
            _ => break,
        }
    }
    panic!("the game never offered seat 0 a land to play");
}

/// The whole of arm-then-act: the first tap says nothing, the second
/// sends, and cancel leaves the wire empty.
///
/// A window offering one land, one spell, and one card that is both.
///
/// Synthetic rather than dealt, because what is under test is the click
/// path and a real deck cannot be relied on to hold a castable spell and a
/// modal double-faced land in the same opening hand.
fn window_with(lands: Vec<ObjectId>, castable: Vec<ObjectId>) -> crate::Duel {
    crate::Duel {
        interaction: Some(Interaction::new(
            Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(LegalActions {
                    can_pass: true,
                    lands,
                    castable,
                    mana_abilities: vec![],
                    abilities: vec![],
                    suspendable: vec![],
                }),
            },
            PlayerId::new(0),
        )),
        ..Default::default()
    }
}

/// A tap on the top card of a pile opens that pile.
///
/// This is the only way in to a graveyard now that the pile chips are
/// gone, so it needs a witness rather than a reading: `open_pile` was
/// wired before the chips were removed and nothing ever clicked it, which
/// is precisely the shape of defect this client has shipped before. The
/// door has to be proved from a *tap* — `activate_card`, the same
/// function the pointer calls — and not by calling `open_pile` directly,
/// or the test would pass with the last branch of `activate_card` gone.
#[test]
fn a_tap_on_a_pile_opens_it() {
    use baylee_client_core::test_support::{ViewBuilder, printed};

    let top = obj(7);
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(7, 0, "Llanowar Elves", 1)])
        .build();
    let mut duel = crate::Duel::default();
    duel.receive_view(view);
    crate::rebuild_board(&mut duel);
    assert!(!duel.browser.is_open(), "nothing has been tapped yet");

    super::activate_card(&mut duel, top);
    assert!(duel.browser.is_open(), "the graveyard did not open");
    assert_eq!(
        duel.browser.ticked().iter().copied().collect::<Vec<_>>(),
        vec![baylee_client_core::BrowseZone::Graveyard(PlayerId::new(0))],
        "it opened on somebody else's pile"
    );
}

/// And a library never opens, however often it is tapped: nobody may look
/// through one, their own included (CR 401.2), so the pile beside the mat
/// is inert rather than merely empty.
///
/// The pile is **manufactured**, because a view cannot produce one: a
/// library is face down to everybody, so `ZonePile::top` is always `None`
/// there and no tap can ever name its card. It is built anyway because a
/// test that tapped an object on no pile at all would pass with every
/// guard in `open_pile` deleted, and would then be claiming CR 401.2 while
/// holding nothing. Two independent readings refuse it — `is_browsable`
/// and `BrowseZone::of_pile` — and each has its own witness in
/// `baylee-client-core`; what is asserted here is the outcome a player
/// sees.
#[test]
fn a_tap_on_a_library_opens_nothing() {
    use baylee_client_core::layout::PileKind;
    use baylee_client_core::test_support::ViewBuilder;

    let top = obj(7);
    let view = ViewBuilder::new(2).build();
    let mut duel = crate::Duel::default();
    duel.receive_view(view);
    crate::rebuild_board(&mut duel);
    {
        let board = duel.board.as_mut().expect("the view built a board");
        let pile = board.pods[0]
            .piles
            .iter_mut()
            .find(|pile| pile.kind == PileKind::Library)
            .expect("every seat has a library");
        pile.count = 60;
        pile.top = Some(top);
    }

    super::activate_card(&mut duel, top);
    assert!(!duel.browser.is_open());
}

#[test]
fn a_land_plays_on_the_click() {
    let land = obj(3);
    let mut duel = window_with(vec![land], vec![]);

    super::activate_card(&mut duel, land);
    assert_eq!(
        duel.outbox(),
        [PlayerAction::PlayLand { card: land }],
        "the land did not play on the click"
    );
    assert!(duel.armed.is_none(), "a land asked for a confirmation");
}

/// There is no undo in the engine and there should not be one, so the
/// client owes a player the chance to take a tap back before it becomes a
/// game action. Tested at this level and not on `Interaction`, because
/// what is being claimed is about *taps*: an assertion that the second
/// call to a resolver returns an action would pass just as well if the
/// first one had already sent it.
#[test]
fn a_spell_still_arms_and_a_second_tap_sends_it() {
    let spell = obj(4);
    let mut duel = window_with(vec![], vec![spell]);

    super::activate_card(&mut duel, spell);
    assert!(
        duel.outbox().is_empty(),
        "the first tap put a spell on the wire"
    );
    assert_eq!(
        duel.armed,
        Some(crate::Armed {
            object: spell,
            deed: crate::Deed::Play
        })
    );

    super::activate_card(&mut duel, spell);
    assert_eq!(duel.outbox(), [PlayerAction::CastSpell { card: spell }]);
    assert!(duel.armed.is_none(), "firing left the deed armed");
}

/// The exception to the exception. A modal double-faced card with a spell
/// front and a land back is in *both* lists, and `play_card` checks lands
/// first — so one-clicking it would resolve it to "play as land" every
/// time and the front face would be unreachable by mouse.
#[test]
fn a_card_that_is_both_a_land_and_a_spell_does_not_one_click() {
    let mdfc = obj(5);
    let mut duel = window_with(vec![mdfc], vec![mdfc]);

    super::activate_card(&mut duel, mdfc);
    assert!(
        duel.outbox().is_empty(),
        "a card with two ways to play it fired one of them on the click"
    );
    assert!(duel.armed.is_some());
}

/// A duel sitting on a `ChooseCards { min: 0 }` with the dialog open on
/// it — a Solemn Simulacrum's search for a basic land, which is the
/// shape AE6 was seen in twice.
fn duel_searching(min: u8) -> crate::Duel {
    use baylee_engine::choice::ChoicePrompt;
    let mut duel = crate::Duel {
        interaction: Some(Interaction::new(
            Pending::ChooseCards {
                player: PlayerId::new(0),
                options: vec![obj(1), obj(2)],
                min,
                max: 1,
                prompt: ChoicePrompt::Generic,
            },
            PlayerId::new(0),
        )),
        ..Default::default()
    };
    // Through the real door: `follow` is what opens the sheet for a
    // question, and `answers_here` is false for a sheet opened any other
    // way — so poking the state would test a configuration the client
    // cannot reach.
    let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    // Destructured so the two fields are borrowed apart: `Interaction`
    // is not `Clone`, and it should not become one for a test.
    let crate::Duel {
        browser,
        interaction,
        ..
    } = &mut duel;
    browser.follow(&view, interaction.as_ref());
    assert!(
        duel.browser.answers_here(duel.interaction.as_ref()),
        "the dialog is the surface holding the question"
    );
    duel
}

fn press(name: bevy::prelude::KeyCode) -> bevy::input::ButtonInput<bevy::prelude::KeyCode> {
    let mut keys = bevy::input::ButtonInput::default();
    keys.press(name);
    keys
}

/// AE6, and the whole reason the dialog has a keyboard of its own.
///
/// The confirm key is Space and means two things — "I am done here" and
/// "pass priority". Three triggers go on the stack, the player passes
/// through them, and a `ChooseCards { min: 0 }` arrives mid-rhythm: the
/// next press answered it with "nothing found", with no trace and no
/// undo, and the land the search was for never entered play.
///
/// `docs/redesign-proposal.md` §6 says Space toggles in this dialog, and
/// that is also the fix: a stray press now does something visible that
/// the same key takes back.
#[test]
fn the_confirm_key_cannot_throw_a_search_away() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::Keymap;

    let mut duel = duel_searching(0);
    let keymap = Keymap::standard();
    let keys = press(bevy::prelude::KeyCode::Space);

    assert!(super::browser_answer_keys(
        Fired::of(&keys, &keymap),
        &mut duel
    ));
    assert!(
        duel.outbox().is_empty(),
        "the search was answered with nothing"
    );
    let it = duel.interaction.as_ref().expect("the question stands");
    assert!(it.is_selected(obj(1)), "the focused row was ticked instead");
    // And the same key again takes it back, which is what makes the
    // stray press harmless rather than merely slower.
    assert!(super::browser_answer_keys(
        Fired::of(&keys, &keymap),
        &mut duel
    ));
    let it = duel
        .interaction
        .as_ref()
        .expect("the question still stands");
    assert!(!it.is_selected(obj(1)));
    assert!(duel.outbox().is_empty());
}

/// The counter-test, because "Space does nothing now" would pass the one
/// above: with no dialog holding the question the key is a pass again.
#[test]
fn the_confirm_key_still_passes_priority_with_no_dialog_up() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::Keymap;

    let mut duel = window_with(vec![], vec![]);
    let keymap = Keymap::standard();
    let keys = press(bevy::prelude::KeyCode::Space);
    let fired = Fired::of(&keys, &keymap);

    assert!(
        !super::browser_answer_keys(fired, &mut duel),
        "no dialog, so the dialog's keyboard declines the frame"
    );
    let mut prefs = crate::prefs::Prefs::default();
    super::answer_the_question(fired, &mut duel, &mut prefs);
    assert_eq!(duel.outbox(), &[PlayerAction::PassPriority]);
}

/// And a search that *must* take a card is no different: the key was
/// never able to answer that one, and it still ticks rather than
/// reaching past the dialog to a confirm that would be refused.
#[test]
fn a_search_with_a_minimum_is_ticked_by_the_same_key() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::Keymap;

    let mut duel = duel_searching(1);
    let keymap = Keymap::standard();
    let keys = press(bevy::prelude::KeyCode::Space);

    assert!(super::browser_answer_keys(
        Fired::of(&keys, &keymap),
        &mut duel
    ));
    let it = duel.interaction.as_ref().expect("the question stands");
    assert!(it.is_selected(obj(1)));
    assert!(duel.outbox().is_empty(), "ticking is not sending");
}

/// §6 gives the dialog Enter, and the table kept taking it.
///
/// `the_click` answers the card under the pointer before anything else,
/// and a card under the pointer is the ordinary state of a table with a
/// sheet standing over it — the permanent whose ability asked the
/// question is usually the very card the pointer is resting on. So the
/// one key the dialog needs was the one key it was least likely to get,
/// and the press went to the table instead, silently.
///
/// The second half of the test is what says the precedence matters: the
/// same press, on the same duel, is taken by `the_click` and spent on a
/// card that is not even part of the question.
#[test]
fn the_dialog_answers_enter_rather_than_the_card_under_the_pointer() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::Keymap;

    let keymap = Keymap::standard();
    let space = press(bevy::prelude::KeyCode::Space);
    let enter = press(bevy::prelude::KeyCode::Enter);

    let mut duel = duel_searching(1);
    // A row ticked, and the pointer left on something else entirely —
    // the fetchland that asked the question, lying in the graveyard.
    super::browser_answer_keys(Fired::of(&space, &keymap), &mut duel);
    duel.hovered = Some(obj(7));

    assert!(
        super::browser_answer_keys(Fired::of(&enter, &keymap), &mut duel),
        "the dialog takes the frame, so the dispatch never reaches the table"
    );
    assert_eq!(
        duel.outbox(),
        &[PlayerAction::ChooseObjects {
            objects: vec![obj(1)]
        }],
        "Enter sent the answer the player had built"
    );

    // And what that precedence is holding back.
    let mut table = duel_searching(1);
    table.hovered = Some(obj(7));
    let mut prefs = crate::prefs::Prefs::default();
    assert!(
        super::the_click(Fired::of(&enter, &keymap), &mut table, &mut prefs),
        "the hovered card would have eaten the key"
    );
    assert!(
        table.outbox().is_empty(),
        "…and answered nothing with it, which is how the press vanished"
    );
}

/// Cancel is the whole point of arming: it has to leave nothing behind.
#[test]
fn cancel_disarms_with_nothing_on_the_wire() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::{Action, Chord, Keymap};
    use bevy::prelude::KeyCode;

    // A spell, because a land no longer arms at all.
    let spell = obj(4);
    let mut duel = window_with(vec![], vec![spell]);

    super::activate_card(&mut duel, spell);
    assert!(duel.armed.is_some());

    let mut keymap = Keymap::standard();
    keymap.bind(Action::Cancel, vec![Chord::key("Escape")]);
    let mut keys = bevy::input::ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::Escape);
    assert!(super::armed_keys(Fired::of(&keys, &keymap), &mut duel));
    assert!(duel.armed.is_none(), "cancel left the deed armed");
    assert!(duel.outbox().is_empty(), "cancel sent something");
}

/// The exception, and the reason it is one: floating mana is the cheap
/// mistake in this game, so tapping a land stays a single tap.
#[test]
fn a_mana_ability_still_goes_through_on_one_tap() {
    use crate::host::DuelHost;
    let (mut duel, mut host) = duel_in_main_phase();
    let land = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .and_then(|l| l.lands.first().copied())
        .expect("the window offers a land");

    // Put the land in play first — it is the only mana source this deck
    // has, and a land in hand makes no mana. One call: a land plays on
    // the click now, and a second would send `PlayLand` twice.
    super::activate_card(&mut duel, land);
    // `outbox` is private to `Duel` but declared in the crate root, so a
    // child module may drain it — which is what `flush_outbox` does in
    // the running client.
    for action in std::mem::take(&mut duel.outbox) {
        host.submit(action);
    }
    for message in host.poll() {
        match message {
            crate::host::HostMessage::View(v) => duel.view = Some(*v),
            crate::host::HostMessage::Choice(p) => {
                duel.interaction = Some(Interaction::new(*p, PlayerId::new(0)));
            }
            _ => {}
        }
    }
    let source = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .and_then(|l| l.mana_abilities.first().copied())
        .expect("the land that was just played can be tapped");

    super::activate_card(&mut duel, source);
    assert!(
        duel.armed.is_none(),
        "tapping for mana asked for a confirmation"
    );
    assert_eq!(
        duel.outbox(),
        [PlayerAction::ActivateManaAbility { source }]
    );
}
