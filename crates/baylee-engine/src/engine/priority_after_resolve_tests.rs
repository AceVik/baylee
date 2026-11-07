//! The active player receives priority after a spell resolves (CR 117.3b),
//! played with real cards in the declare blockers step.
//!
//! The report (llm 01a1b399, 7.11.2026): P2 attacked with two Elvish
//! Lookouts, one blocked by Wee Dragonauts, cast Plummet on the blocker and
//! passed so it resolved; combat damage followed with no question. The
//! engine did give P2 priority back, but there was nothing to do with it:
//! Colossal Might (instant, "target creature") had no legal target left,
//! because both Lookouts have shroud (CR 702.18a) and the only other
//! creature had died. The same table with an attacker without shroud shows
//! the priority is asked and the Might is on offer.

use super::testkit::*;
use super::*;
use crate::turn::Step;
use baylee_core::ids::{CardIndex, Defender};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn elvish_lookout() -> CardIndex {
    card_index("9e9aea99-1eee-4bb6-9043-a1ac2884d071")
}
fn wee_dragonauts() -> CardIndex {
    card_index("e9e56b1d-5ad2-4807-8c4c-1a1a6a25ecbf")
}
fn plummet() -> CardIndex {
    card_index("85bde6ac-3dd4-4946-8b57-24f57e3eae2b")
}
fn colossal_might() -> CardIndex {
    card_index("1d8e39c9-e04d-4328-af66-db6e1241ac06")
}
fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn holds_priority(engine: &Engine<RegistryLookup>, seat: PlayerId) -> bool {
    matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
}

/// Seat 0 attacks with `attacker`, seat 1 blocks it with Wee Dragonauts, and
/// seat 0 casts Plummet on the blocker and passes. Returns the engine once
/// Plummet has resolved.
fn plummet_resolves_in_blockers(attacker: CardIndex) -> Engine<RegistryLookup> {
    let mut board = vec![attacker];
    board.extend([
        basic_forest(),
        basic_forest(),
        basic_forest(),
        mountain(),
        mountain(),
    ]);
    let mut engine = Duel::new(0x0011_73b0, basic_forest())
        .battlefield(0, &board)
        .battlefield(1, &[wee_dragonauts()])
        .hand(0, &[plummet(), colossal_might()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseAttackers { .. } => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    let attacker_id = on_battlefield(&engine, P0, attacker).expect("the attacker");
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker_id, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let dragonauts = on_battlefield(&engine, P1, wee_dragonauts()).expect("the blocker");
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(dragonauts, attacker_id)],
            },
        )
        .unwrap();
    // The active player has priority in the declare blockers step (CR 509.4).
    assert!(holds_priority(&engine, P0), "{:?}", engine.pending());
    assert_eq!(engine.state().turn.step, Step::DeclareBlockers);

    cast_from_hand(&mut engine, P0, plummet());
    for _ in 0..10 {
        if let Pending::ChooseTargets { options, .. } = engine.pending().clone() {
            assert!(options.contains(&dragonauts));
            engine
                .apply(
                    P0,
                    PlayerAction::ChooseObjects {
                        objects: vec![dragonauts],
                    },
                )
                .unwrap();
        }
    }
    assert!(holds_priority(&engine, P0), "{:?}", engine.pending());
    // The caster passes, the opponent passes, Plummet resolves.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(holds_priority(&engine, P1), "{:?}", engine.pending());
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert!(
        on_battlefield(&engine, P1, wee_dragonauts()).is_none(),
        "Plummet destroyed the flier"
    );
    engine
}

/// CR 117.3b: the active player is asked again, in the same step, with the
/// stack empty and no damage dealt; a creature without shroud can take
/// Colossal Might.
#[test]
fn the_active_player_has_priority_again_after_the_spell_resolves() {
    let engine = plummet_resolves_in_blockers(quiet_creature());
    assert!(holds_priority(&engine, P0), "{:?}", engine.pending());
    assert_eq!(engine.state().turn.step, Step::DeclareBlockers);
    assert!(stack_is_empty(&engine));
    assert_eq!(engine.state().players[1].life, 20);
    let might = in_hand(&engine, P0, colossal_might()).expect("the Might");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(legal.castable.contains(&might), "{legal:?}");
}

/// The reported table: the attacker has shroud, so after the Dragonauts die
/// Colossal Might has no target (CR 702.18a, 601.2c). Priority is still
/// given, with nothing castable.
#[test]
fn a_shroud_attacker_leaves_the_might_without_a_target() {
    let engine = plummet_resolves_in_blockers(elvish_lookout());
    assert!(holds_priority(&engine, P0), "{:?}", engine.pending());
    assert_eq!(engine.state().turn.step, Step::DeclareBlockers);
    let might = in_hand(&engine, P0, colossal_might()).expect("the Might");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(!legal.castable.contains(&might), "{legal:?}");
}
