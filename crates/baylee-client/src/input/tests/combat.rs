//! Declaring an attack, and the key that must not decline one.
//!
//! The two combat windows are the only place on the table where the confirm
//! key sends something a player cannot take back and did not ask for. It was
//! found by playing rather than by reading: three presses of `Space` to walk
//! from a main phase into combat walked *through* the attack step, declared no
//! attackers, and left a 2/1 untapped against an open opponent — no
//! confirmation, no refusal line, and nothing on the wire to undo. Every
//! `Interaction` test stayed green, because declaring nothing is a legal
//! answer and [`Interaction::confirm`] was right to build it.
//!
//! So nothing here is hand-built between the key and the action. Each test
//! presses a `KeyCode` at the real [`keyboard`] system and reads
//! [`crate::Duel::outbox`], which is the only place the difference between
//! "refused" and "sent an empty declaration" is visible.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use baylee_core::ids::Defender;
use baylee_engine::choice::BlockOption;
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

/// A declare-attackers question offering `attackers` as candidates.
fn attackers(candidates: Vec<ObjectId>) -> Pending {
    Pending::ChooseAttackers {
        player: PlayerId::new(0),
        attackers: candidates,
        defenders: vec![Defender::Player(PlayerId::new(1))],
    }
}

/// A declare-blockers question offering `blocker` against one attacker.
fn blockers(candidates: Vec<BlockOption>) -> Pending {
    Pending::ChooseBlockers {
        player: PlayerId::new(0),
        attacker: PlayerId::new(1),
        blockers: candidates,
    }
}

fn asking(pending: Pending) -> crate::Duel {
    crate::Duel {
        interaction: Some(baylee_client_core::interaction::Interaction::new(
            pending,
            PlayerId::new(0),
        )),
        ..Default::default()
    }
}

/// Presses `key` at the real keyboard system and hands back what was sent.
fn press(duel: crate::Duel, key: KeyCode) -> Vec<PlayerAction> {
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
    app.world().resource::<crate::Duel>().outbox().to_vec()
}

/// The precondition every test below rests on, asserted once: the standard
/// keymap is the one where `Space` confirms and `O` declares nothing.
///
/// Without it a test that pressed the wrong key would pass by sending
/// nothing, which is the outcome three of these four are written to see.
#[test]
fn space_confirms_and_o_declines_in_the_standard_map() {
    use baylee_client_core::prefs::Action;

    let prefs = crate::prefs::Prefs::default();
    for (key, action) in [
        (KeyCode::Space, Action::Confirm),
        (KeyCode::KeyO, Action::CombatNone),
    ] {
        let mut probe = ButtonInput::<KeyCode>::default();
        probe.press(key);
        assert!(
            crate::keys::Fired::of(&probe, prefs.keymap()).has(action),
            "{key:?} is {action:?} in the standard map"
        );
    }
}

/// The defect: the confirm key does not spend an attack step.
///
/// Fails against the old code with `[DeclareAttackers { attackers: [] }]`,
/// which is exactly the frame the report describes — the step answered, the
/// creature still untapped, the player none the wiser.
#[test]
fn the_confirm_key_does_not_declare_an_empty_attack() {
    assert_eq!(
        press(asking(attackers(vec![obj(3)])), KeyCode::Space),
        Vec::<PlayerAction>::new(),
        "a creature that could attack must not be skipped by the pass key"
    );
}

/// The same defect on the other side of combat, where it costs a block.
#[test]
fn the_confirm_key_does_not_declare_an_empty_block() {
    let option = BlockOption {
        blocker: obj(3),
        attackers: vec![obj(9)],
    };
    assert_eq!(
        press(asking(blockers(vec![option])), KeyCode::Space),
        Vec::<PlayerAction>::new(),
        "a creature that could block must not be skipped by the pass key"
    );
}

/// The half that would be lost by over-refusing: once something is declared,
/// the confirm key is what sends it.
///
/// This is the test that stops the guard from being written as "the confirm
/// key never commits combat", which would take the answer away from the
/// button the prompt bar draws.
#[test]
fn the_confirm_key_sends_a_declaration_that_stands() {
    let mut duel = asking(attackers(vec![obj(3)]));
    assert!(
        duel.interaction
            .as_mut()
            .is_some_and(|i| i.declare_attacker(obj(3), Defender::Player(PlayerId::new(1)))),
        "the candidate the question offered is declarable"
    );
    assert_eq!(
        press(duel, KeyCode::Space),
        vec![PlayerAction::DeclareAttackers {
            attackers: vec![(obj(3), Defender::Player(PlayerId::new(1)))],
        }],
        "the declaration standing is what the confirm key sends"
    );
}

/// Declining is still a real answer, and `O` is still how it is given.
///
/// `declare_nothing` reaches [`Interaction::confirm`] directly rather than
/// through the guard, and this is what says so: the empty declaration the
/// confirm key is refused is the very action this key exists to send.
#[test]
fn the_decline_key_still_declares_no_attackers() {
    assert_eq!(
        press(asking(attackers(vec![obj(3)])), KeyCode::KeyO),
        vec![PlayerAction::DeclareAttackers { attackers: vec![] }],
        "`O` is the answer the confirm key gave up"
    );
}

/// With nothing to declare, the confirm key walks through as it always did.
///
/// The guard is conditional on there being something to lose, and this is the
/// condition. A `ChooseAttackers` with no legal attacker is a question with
/// one possible answer — the engine asks it anyway — and refusing the key
/// there would stop a player's rhythm at a step that could never have held an
/// attack. Written down because the predicate reads more simply without this
/// clause, and simplifying it would cost exactly this.
#[test]
fn a_combat_step_with_no_candidate_still_answers_to_the_confirm_key() {
    assert_eq!(
        press(asking(attackers(vec![])), KeyCode::Space),
        vec![PlayerAction::DeclareAttackers { attackers: vec![] }],
        "nothing could attack, so nothing is lost by saying so"
    );
}
