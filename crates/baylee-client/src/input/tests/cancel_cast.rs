//! Taking a cast back (CR 732; the engine's `PlayerAction::CancelCast`,
//! 08.10.2026): `Esc`'s last rung and the shelf's last answer, while the
//! view names the cast, and nowhere else.

#[allow(clippy::wildcard_imports)] // the input tests' shared fixtures
use super::*;

/// A target question of a spell this seat is casting (object 90), the
/// cast-first run waiting on it; `casting` is what the view says.
fn casting(named: bool) -> crate::Duel {
    let spell = obj(90);
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    view.casting = named.then_some(spell);
    let mut duel = crate::Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            Pending::ChooseTargets {
                player: PlayerId::new(0),
                options: vec![obj(30)],
                player_options: vec![PlayerId::new(1)],
                min: 1,
                max: 1,
                reason: baylee_engine::choice::TargetPrompt::Targets,
            },
            PlayerId::new(0),
        )),
        mana_run: Some(crate::ManaRun::cast_first(spell).sent()),
        ..Default::default()
    };
    duel.view = Some(view);
    duel
}

/// `duel` after one press of `key` through the real keyboard system.
fn after_pressing(duel: crate::Duel, key: bevy::prelude::KeyCode) -> crate::Duel {
    use bevy::input::ButtonInput;
    use bevy::input::keyboard::KeyboardInput;
    use bevy::prelude::*;
    let mut app = App::new();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<crate::table::CameraRig>()
        .init_resource::<crate::settings::ClientSettings>()
        .add_message::<KeyboardInput>()
        .insert_resource(duel)
        .add_systems(Update, keyboard);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.update();
    app.world_mut()
        .remove_resource::<crate::Duel>()
        .expect("the duel")
}

fn cancelled(duel: &crate::Duel) -> bool {
    duel.outbox()
        .iter()
        .any(|a| matches!(a, PlayerAction::CancelCast))
}

/// With the cast named and nothing nearer to take back, `Esc` takes the
/// cast back — and ends the run that was waiting to pay for it, which
/// would otherwise tap the lands the engine just untapped.
#[test]
fn escape_takes_a_named_cast_back_and_ends_its_run() {
    let duel = after_pressing(casting(true), bevy::prelude::KeyCode::Escape);
    assert!(cancelled(&duel), "sent: {:?}", duel.outbox());
    assert!(duel.mana_run.is_none(), "the run went with the cast");
}

/// The view names no cast (an effect's cast, a mana ability's colour in
/// the window): `Esc` sends nothing of the kind — red if the client
/// guessed a cast from the question alone.
#[test]
fn escape_takes_back_no_cast_the_view_does_not_name() {
    let duel = after_pressing(casting(false), bevy::prelude::KeyCode::Escape);
    assert!(!cancelled(&duel), "sent: {:?}", duel.outbox());
}

/// Innermost first: a half-built answer is taken back before the cast.
#[test]
fn escape_clears_a_chosen_target_before_the_cast() {
    let mut duel = casting(true);
    if let Some(i) = duel.interaction.as_mut() {
        i.toggle(obj(30));
    }
    let duel = after_pressing(duel, bevy::prelude::KeyCode::Escape);
    assert!(!cancelled(&duel), "the first Esc took the cast back");
    assert!(
        duel.interaction
            .as_ref()
            .is_some_and(|i| i.selected().next().is_none()),
        "and did not clear the target"
    );
    let duel = after_pressing(duel, bevy::prelude::KeyCode::Escape);
    assert!(cancelled(&duel), "the second takes the cast back");
}

/// The shelf's button is the same door.
#[test]
fn the_cancel_button_takes_the_cast_back() {
    let mut app = pointer_app(casting(true));
    let button = app
        .world_mut()
        .spawn(crate::hud::MenuButton {
            action: crate::hud::MenuAction::CancelCast,
        })
        .id();
    click(&mut app, button);
    let duel = app.world().resource::<crate::Duel>();
    assert!(cancelled(duel), "sent: {:?}", duel.outbox());
    assert!(duel.mana_run.is_none());
}
