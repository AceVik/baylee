//! Camouflage: blocks made by piles assigned at random (`engine/camouflage.rs`).

use super::testkit::*;
use super::*;
use crate::choice::ChoicePrompt;
use crate::event::GameEvent;
use baylee_core::generated::index;
use baylee_core::ids::{CardIndex, Defender};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// P0 attacks P1 with its two creatures, casts Camouflage in its declare
/// attackers step, and the game runs to P1's first pile question.
fn camouflaged(seed: u64, mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut board = mine.to_vec();
    board.push(index::FOREST);
    let mut engine = Duel::new(seed, index::FOREST)
        .battlefield(0, &board)
        .hand(0, &[index::CAMOUFLAGE])
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let attackers: Vec<ObjectId> = mine
        .iter()
        .map(|&card| on_battlefield(&engine, P0, card).unwrap())
        .collect();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&a| (a, Defender::Player(P1)))
                    .collect(),
            },
        )
        .unwrap();
    cast_from_hand(&mut engine, P0, index::CAMOUFLAGE);
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            _ => break,
        }
    }
    engine
}

fn pile_question(engine: &Engine<RegistryLookup>) -> Option<(u8, u8, Vec<ObjectId>)> {
    match engine.pending() {
        Pending::ChooseCards {
            player,
            options,
            prompt: ChoicePrompt::CamouflagePile { pile, of },
            min: 0,
            ..
        } if *player == P1 => Some((*pile, *of, options.clone())),
        _ => None,
    }
}

fn blocks(engine: &Engine<RegistryLookup>) -> Vec<(ObjectId, ObjectId)> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::BecameBlocker { object, attacker } => Some((object, attacker)),
            _ => None,
        })
        .collect()
}

/// Instead of declaring blockers, P1 names two piles (two attackers), one
/// creature in each; the piles go to different attackers at random and each
/// creature blocks its pile's attacker. The draw is the game's seeded one:
/// the same seed deals the same blocks, and across seeds both deals occur.
#[test]
fn piles_are_assigned_to_different_attackers_at_random_and_block() {
    let mut seen = std::collections::BTreeSet::new();
    for seed in 8101..8121 {
        let play = |seed| {
            let mut engine = camouflaged(
                seed,
                &[index::GRIZZLY_BEARS, index::HILL_GIANT],
                &[index::GRIZZLY_BEARS, index::HILL_GIANT],
            );
            let wall = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
            let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
            let (pile, of, options) = pile_question(&engine).expect("the first pile");
            assert_eq!((pile, of), (1, 2), "one pile per attacker");
            assert!(options.contains(&wall) && options.contains(&giant));
            engine
                .apply(
                    P1,
                    PlayerAction::ChooseObjects {
                        objects: vec![wall],
                    },
                )
                .unwrap();
            let (pile, _, options) = pile_question(&engine).expect("the second pile");
            assert_eq!(pile, 2);
            assert_eq!(options, vec![giant], "a creature goes into one pile");
            engine
                .apply(
                    P1,
                    PlayerAction::ChooseObjects {
                        objects: vec![giant],
                    },
                )
                .unwrap();
            let made = blocks(&engine);
            assert_eq!(made.len(), 2, "both block: {made:?}");
            assert_ne!(made[0].1, made[1].1, "different attackers");
            made
        };
        let first = play(seed);
        assert_eq!(first, play(seed), "seeded: the same deal again");
        seen.insert(first);
    }
    assert_eq!(seen.len(), 2, "both deals occur: {seen:?}");
}

/// A creature blocks its pile's attacker only if it can: P1's Bears in the
/// only nonempty pile never block P0's Serra Angel (flying), whichever
/// attacker the pile went to.
#[test]
fn a_creature_that_cannot_block_its_piles_attacker_does_not() {
    let mut blocked_bears = false;
    for seed in 8201..8221 {
        let mut engine = camouflaged(
            seed,
            &[index::SERRA_ANGEL, index::HILL_GIANT],
            &[index::GRIZZLY_BEARS],
        );
        let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
        let angel = on_battlefield(&engine, P0, index::SERRA_ANGEL).unwrap();
        engine
            .apply(
                P1,
                PlayerAction::ChooseObjects {
                    objects: vec![bears],
                },
            )
            .unwrap();
        let made = blocks(&engine);
        assert!(!made.contains(&(bears, angel)), "it cannot block a flier");
        blocked_bears |= !made.is_empty();
    }
    assert!(
        blocked_bears,
        "and when the pile went to the Giant, it blocked"
    );
}

/// "Cast this spell only during your declare attackers step": not in a main
/// phase.
#[test]
fn camouflage_is_cast_only_during_your_declare_attackers_step() {
    let mut engine = Duel::new(8301, index::FOREST)
        .battlefield(0, &[index::FOREST])
        .hand(0, &[index::CAMOUFLAGE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, index::CAMOUFLAGE).unwrap();
    assert!(engine.apply(P0, PlayerAction::CastSpell { card }).is_err());
}
