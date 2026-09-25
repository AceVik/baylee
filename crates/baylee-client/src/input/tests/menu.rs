//! What a seat says when nobody has asked it anything. A draw offer, a concession and a priority hold are statements rather than answers, so each carries its own guard: a draw only from this seat's own priority, because the engine refuses the rest; a concession only on a second press with nothing in between; and a hold that either key *takes back* rather than replaces, because a player who has stopped being asked should not have to remember which one they pressed. Both doors to each are held here — the `MenuButton` through the real `pointer` and `menu_click` straight — since an empty prompt bar is also what an idle turn looks like, and the outbox is the only place a hold and a pass differ at all. The zone panel's own controls are `tray`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

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
            HostMessage::View(v, _) => Some(*v),
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
        .add_systems(Update, keyboard);

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
            HostMessage::View(v, _) => Some(*v),
            _ => None,
        })
        .expect("a view");
    view.priority_held = true;
    let mut duel = crate::Duel {
        view: Some(view),
        ..Default::default()
    };

    menu_click(&mut duel, MenuAction::ReleaseHold, false);
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
    menu_click(&mut fresh, MenuAction::ReleaseHold, false);
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
            HostMessage::View(v, _) => Some(*v),
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
    menu_click(&mut duel, MenuAction::HoldForStack, false);
    assert_eq!(
        duel.outbox(),
        [PlayerAction::SetPriorityHold(
            PriorityHold::UntilStackEmpty { depth: 1 }
        )]
    );

    // Nothing on the stack: the same press would ask for a hold that is
    // already over, so it asks for nothing at all.
    let mut nothing = seated(false, false);
    menu_click(&mut nothing, MenuAction::HoldForStack, false);
    assert!(nothing.outbox().is_empty());

    // And with one already running it sends nothing either, rather than
    // the `Always` that would end it — cancelling is `ReleaseHold`'s job.
    let mut running = seated(true, true);
    menu_click(&mut running, MenuAction::HoldForStack, false);
    assert!(running.outbox().is_empty());
}

/// `Esc` walks the screen from the top down, and the game menu is a rung on
/// that ladder rather than something only its own button can shut.
///
/// It stands **above** the zone browser and below the preview, which is the
/// only part of the order that had to be decided: both can be up at once, and
/// the browser is the one that can stand open for a whole turn while nobody
/// opens the menu and then forgets it. The counter-test is the same press
/// with no menu open — "Escape now closes nothing" would pass the first half
/// on its own.
#[test]
fn escape_puts_the_game_menu_away_before_it_reaches_the_browser() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::Keymap;

    let keymap = Keymap::standard();
    let escape = Fired::of(&press(bevy::prelude::KeyCode::Escape), &keymap);
    let mut prefs = crate::prefs::Prefs::default();

    let mut duel = crate::Duel::default();
    duel.browser.open();
    duel.game_menu = true;

    answer_the_question(escape, &mut duel, &mut prefs);
    assert!(!duel.game_menu, "the menu was the top thing on the screen");
    assert!(
        duel.browser.is_open(),
        "and the press stopped there: a panel and the sheet behind it must \
         not both go on one key"
    );

    answer_the_question(escape, &mut duel, &mut prefs);
    assert!(
        !duel.browser.is_open(),
        "the next press reaches the browser"
    );

    // The counter-test.
    let mut duel = crate::Duel::default();
    duel.browser.open();
    answer_the_question(escape, &mut duel, &mut prefs);
    assert!(
        !duel.browser.is_open(),
        "with no menu up the browser is what the first press closes, which is \
         what this test asserts the menu got in front of"
    );
}

/// The game log is a rung on the same ladder: under the menu, which the
/// player opened over it, and over the browser, which a question may be
/// holding.
#[test]
fn escape_puts_the_log_away_between_the_menu_and_the_browser() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::Keymap;

    let keymap = Keymap::standard();
    let escape = Fired::of(&press(bevy::prelude::KeyCode::Escape), &keymap);
    let mut prefs = crate::prefs::Prefs::default();

    let mut duel = crate::Duel::default();
    duel.browser.open();
    duel.game_menu = true;
    duel.log_open = true;

    answer_the_question(escape, &mut duel, &mut prefs);
    assert!(!duel.game_menu, "the menu was the top thing on the screen");
    assert!(duel.log_open, "and one press put two panels away");
    answer_the_question(escape, &mut duel, &mut prefs);
    assert!(!duel.log_open, "the next press reaches the log");
    assert!(duel.browser.is_open(), "and stops there");
    answer_the_question(escape, &mut duel, &mut prefs);
    assert!(!duel.browser.is_open(), "the browser is last");
}

/// `L` opens the log and shuts it, the way `G` does the browser.
#[test]
fn the_log_key_opens_the_log_and_shuts_it() {
    use crate::keys::Fired;
    use baylee_client_core::prefs::Keymap;

    let keymap = Keymap::standard();
    let l = Fired::of(&press(bevy::prelude::KeyCode::KeyL), &keymap);
    let mut duel = crate::Duel::default();
    let mut rig = crate::table::CameraRig::default();
    let mut settings = crate::settings::ClientSettings::default();
    let mut prefs = crate::prefs::Prefs::default();

    crate::input::look_around(l, &mut duel, &mut rig, &mut settings, &mut prefs);
    assert!(duel.log_open, "L did not open the log");
    crate::input::look_around(l, &mut duel, &mut rig, &mut settings, &mut prefs);
    assert!(!duel.log_open, "L did not shut it again");
}

/// A press anywhere else puts the menu away, and a press on the panel, on a
/// row inside it or on the burger does not.
///
/// The burger is the whole reason the system has an exclusion list at all. A
/// press and a click land on **different frames** — picking turns a press and
/// a release over one entity into a `Pointer<Click>` — so this system sees
/// the press first and would shut the panel, and `menu_click` would read the
/// toggle a frame later and open it again. The burger would have stopped
/// working while still looking like it was doing something.
///
/// Three spares and not one, because they are spared for two different
/// reasons — the row by its **lineage** and the burger by its **action** —
/// and either of them working alone would hide the other being broken.
#[test]
fn a_press_outside_the_panel_puts_it_away_and_three_presses_do_not() {
    use crate::hud::{MenuAction, MenuButton, MenuPanel};
    use bevy::input::ButtonInput;
    use bevy::picking::backend::HitData;
    use bevy::picking::hover::HoverMap;
    use bevy::picking::pointer::PointerId;
    use bevy::prelude::*;

    /// Which of the four things the press landed on.
    enum Aim {
        Table,
        Panel,
        Row,
        Burger,
    }

    let pressed_on = |aim: &Aim| {
        let mut app = App::new();
        app.insert_resource(crate::Duel {
            game_menu: true,
            ..Default::default()
        })
        .add_systems(Update, crate::input::close_the_menu_on_a_press_outside_it);

        let panel = app.world_mut().spawn(MenuPanel).id();
        let row = app
            .world_mut()
            .spawn((
                MenuButton {
                    action: MenuAction::Concede,
                },
                ChildOf(panel),
            ))
            .id();
        let burger = app
            .world_mut()
            .spawn(MenuButton {
                action: MenuAction::ToggleGameMenu,
            })
            .id();
        let table = app.world_mut().spawn_empty().id();
        let camera = app.world_mut().spawn_empty().id();

        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Left);
        app.insert_resource(mouse);

        let target = match aim {
            Aim::Table => table,
            Aim::Panel => panel,
            Aim::Row => row,
            Aim::Burger => burger,
        };
        let mut hovers = HoverMap::default();
        hovers.insert(
            PointerId::Mouse,
            [(target, HitData::new(camera, 0.0, None, None))]
                .into_iter()
                .collect(),
        );
        app.insert_resource(hovers);
        app.update();
        app.world().resource::<crate::Duel>().game_menu
    };

    assert!(
        !pressed_on(&Aim::Table),
        "a press on the table puts the menu away"
    );
    for (aim, what) in [
        (Aim::Panel, "the panel"),
        (Aim::Row, "a row inside the panel"),
        (Aim::Burger, "the burger"),
    ] {
        assert!(
            pressed_on(&aim),
            "a press on {what} closed the menu, and it must not"
        );
    }
}
