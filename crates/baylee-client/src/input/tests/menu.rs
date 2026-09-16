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
            HostMessage::View(v) => Some(*v),
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
