//! Which steps hand a player priority, asked of a whole turn rather than of
//! one rule at a time.
//!
//! Every other test here reaches a window it needs and works from there, so
//! the *shape* of a turn — six windows, in these steps and not those — was
//! never asserted by anything. That is exactly the question a player asks
//! when a game does not stop where they expected it to (#130), and it has
//! two halves that fail differently: a step that stops when the rules say
//! nothing happens there is noise, and a step that does **not** stop is a
//! spell the player was never allowed to cast.

use super::testkit::{
    Duel, RegistryLookup, card_index, cast_from_hand, on_battlefield, walk_to_own_main,
};
use super::*;
use baylee_core::ids::CardIndex;

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// `{0}`, flying, and nothing else: a spell that resolves without asking
/// anything, which is what the resolution half of this file needs.
fn ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

/// One priority window, as the seat that got it and where.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Window {
    active: PlayerId,
    phase: Phase,
    step: Step,
    holder: PlayerId,
}

/// Walks to the end of turn `through`, answering everything the way a player
/// with nothing to do would, and records every priority window on the way.
///
/// It records rather than counts, because the count alone cannot tell the
/// two failures apart: six windows in the wrong six steps is the same number
/// as six in the right ones.
#[track_caller]
fn windows_through(engine: &mut Engine<RegistryLookup>, through: u32) -> Vec<Window> {
    let mut seen = Vec::new();
    for _ in 0..400 {
        if engine.state().turn.number > through {
            return seen;
        }
        let (active, phase, step) = {
            let turn = &engine.state().turn;
            (turn.active, turn.phase, turn.step)
        };
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            Pending::Priority { player, .. } => {
                seen.push(Window {
                    active,
                    phase,
                    step,
                    holder: player,
                });
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("nothing in this game should ask that: {other:?}"),
        }
    }
    panic!("turn {through} never ended");
}

/// CR 503.1 and CR 504.2: the upkeep step and the draw step each give the
/// active player priority, on **every** player's turn.
///
/// The two seats are asserted separately and not as a set, because the
/// interface that reported this stopped in neither step on either turn and a
/// test that merely found one of them somewhere would have passed.
#[test]
fn the_upkeep_and_the_draw_step_hand_priority_to_the_player_whose_turn_it_is() {
    let mut engine = Duel::new(9_130, forest()).start();
    let seen = windows_through(&mut engine, 2);

    for turn_active in [PlayerId::new(0), PlayerId::new(1)] {
        for step in [Step::Upkeep, Step::Draw] {
            let in_step: Vec<Window> = seen
                .iter()
                .copied()
                .filter(|w| w.active == turn_active && w.step == step)
                .collect();
            assert!(
                !in_step.is_empty(),
                "no priority window at all in {step:?} on {turn_active:?}'s turn (CR 117.3a); \
                 the windows were {seen:?}"
            );
            assert_eq!(
                in_step[0].holder, turn_active,
                "the active player is asked first in {step:?} (CR 117.3a)"
            );
            assert!(
                in_step.iter().any(|w| w.holder != turn_active),
                "priority passes to the other seat in {step:?} (CR 117.3d)"
            );
            assert_eq!(
                in_step[0].phase,
                Phase::Beginning,
                "{step:?} belongs to the beginning phase (CR 501.1)"
            );
        }
    }
}

/// CR 502.4 and CR 514.3: the untap step gives nobody priority, and on a
/// board where nothing is waiting neither does the cleanup step.
///
/// The half that makes the test refusable. Without it, an engine that simply
/// offered a window in all twelve steps would pass everything above.
#[test]
fn the_untap_step_and_a_quiet_cleanup_give_nobody_priority() {
    let mut engine = Duel::new(9_131, forest()).start();
    let seen = windows_through(&mut engine, 2);

    assert!(
        !seen.iter().any(|w| w.step == Step::Untap),
        "no player receives priority during the untap step (CR 502.4): {seen:?}"
    );
    assert!(
        !seen.iter().any(|w| w.step == Step::Cleanup),
        "with nothing waiting, the cleanup step asks nobody (CR 514.3): {seen:?}"
    );
}

/// CR 117.3c, then CR 117.3b: the player who cast the spell is asked again
/// before it resolves, and when it has resolved the active player is asked
/// again in the same step.
///
/// This is the observation that looks identical from outside the engine to a
/// client that answered both windows by itself — "I cast a creature and the
/// game went straight to declare attackers" — so it is worth having the
/// engine's own answer written down.
#[test]
fn casting_and_then_resolving_a_spell_both_hand_priority_back_in_the_same_step() {
    let me = PlayerId::new(0);
    let mut engine = Duel::new(9_132, forest()).hand(0, &[ornithopter()]).start();
    assert!(walk_to_own_main(&mut engine, me), "reached my main phase");

    cast_from_hand(&mut engine, me, ornithopter());
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!(
            "the caster is asked again (CR 117.3c), got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, me,
        "the player who cast it is asked first (CR 117.3c)"
    );
    assert_eq!(engine.state().turn.phase, Phase::FirstMain);

    // Both pass, so the spell resolves (CR 117.4).
    engine.apply(me, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("the other seat answers it, got {:?}", engine.pending())
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();

    assert!(
        on_battlefield(&engine, me, ornithopter()).is_some(),
        "the spell resolved"
    );
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!(
            "the active player is asked once it has resolved (CR 117.3b), got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, me,
        "the active player receives priority (CR 117.3b)"
    );
    assert_eq!(
        engine.state().turn.phase,
        Phase::FirstMain,
        "and in the step the spell resolved in, not the next one"
    );
}
