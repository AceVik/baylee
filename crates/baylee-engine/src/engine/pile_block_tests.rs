//! Raging River: piles that restrict blocks (`resolve/river.rs`).
//!
//! "Whenever one or more creatures you control attack, each defending player
//! divides all creatures without flying they control into a 'left' pile and
//! a 'right' pile. Then, for each attacking creature you control, choose
//! 'left' or 'right.' That creature can't be blocked this combat except by
//! creatures with flying and creatures in a pile with the chosen label."

use super::testkit::*;
use super::*;
use crate::choice::ChoicePrompt;
use baylee_core::generated::index;
use baylee_core::ids::Defender;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// P0, with Raging River, attacks P1 with `attackers` (P0's creatures by
/// card) and lets the trigger resolve up to its first question.
fn attack_with(engine: &mut Engine<RegistryLookup>, attackers: &[ObjectId], at: PlayerId) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&a| (a, Defender::Player(at)))
                    .collect(),
            },
        )
        .unwrap();
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            _ => return,
        }
    }
}

fn division_asked_of(engine: &Engine<RegistryLookup>) -> Option<(PlayerId, Vec<ObjectId>)> {
    match engine.pending() {
        Pending::ChooseCards {
            player,
            options,
            prompt: ChoicePrompt::LeftPile,
            min: 0,
            ..
        } => Some((*player, options.clone())),
        _ => None,
    }
}

/// P1 has two creatures without flying and a Serra Angel. P1 divides: the
/// Bears left, the Giant right (the Angel is in no pile, it flies). P0
/// labels its Bears "left" and its Giant "right". Then P1's Bears may block
/// only P0's Bears, P1's Giant only P0's Giant, the Angel either; a block
/// across the river is refused. One trigger for the two attackers.
#[test]
fn each_attacker_may_be_blocked_only_by_fliers_and_its_labelled_pile() {
    let mut engine = Duel::new(7101, index::MOUNTAIN)
        .battlefield(
            0,
            &[index::RAGING_RIVER, index::GRIZZLY_BEARS, index::HILL_GIANT],
        )
        .battlefield(
            1,
            &[index::GRIZZLY_BEARS, index::HILL_GIANT, index::SERRA_ANGEL],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let my_bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let my_giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let their_bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let their_giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    let angel = on_battlefield(&engine, P1, index::SERRA_ANGEL).unwrap();
    attack_with(&mut engine, &[my_bears, my_giant], P1);

    let (who, mut options) = division_asked_of(&engine).expect("P1 divides");
    assert_eq!(who, P1);
    options.sort();
    let mut grounded = vec![their_bears, their_giant];
    grounded.sort();
    assert_eq!(
        options, grounded,
        "the creatures without flying, and no other"
    );
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![their_bears],
            },
        )
        .unwrap();

    for (attacker, pick) in [(my_bears, 0), (my_giant, 1)] {
        let Pending::ChoosePile {
            player,
            piles,
            label,
        } = engine.pending().clone()
        else {
            panic!("a label is asked: {:?}", engine.pending());
        };
        assert_eq!((player, label), (P0, Some(attacker)));
        assert_eq!(piles, vec![vec![their_bears], vec![their_giant]]);
        engine.apply(P0, PlayerAction::ChooseMode(pick)).unwrap();
    }
    assert!(
        division_asked_of(&engine).is_none(),
        "one trigger for the whole attack"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let may = |blocker: ObjectId| -> Vec<ObjectId> {
        let mut a = blockers
            .iter()
            .find(|b| b.blocker == blocker)
            .map(|b| b.attackers.clone())
            .unwrap_or_default();
        a.sort();
        a
    };
    assert_eq!(may(their_bears), vec![my_bears]);
    assert_eq!(may(their_giant), vec![my_giant]);
    let mut both = vec![my_bears, my_giant];
    both.sort();
    assert_eq!(may(angel), both, "flying is never held back");
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(their_bears, my_giant)],
                },
            )
            .is_err(),
        "the Bears are on the other side of the river"
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(their_giant, my_giant), (angel, my_bears)],
            },
        )
        .unwrap();
}

/// Raging River is "creatures you control attack": the opponent's attack
/// asks nothing, and its blocks are unrestricted.
#[test]
fn the_river_does_not_trigger_on_an_opponents_attack() {
    let mut engine = Duel::new(7102, index::MOUNTAIN)
        .battlefield(0, &[index::RAGING_RIVER, index::GRIZZLY_BEARS])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        division_asked_of(e).is_some() || matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(division_asked_of(&engine).is_none());
}

/// At a table of three every opponent of the attacker is a defending player
/// (CR 802.2) and divides, in turn order from the active player
/// (CR 101.4): P1, then P2.
#[test]
fn every_defending_player_divides_in_turn_order() {
    let p2 = PlayerId::new(2);
    let mut engine = Duel::table(7103, index::MOUNTAIN, 3)
        .battlefield(0, &[index::RAGING_RIVER, index::GRIZZLY_BEARS])
        .battlefield(1, &[index::HILL_GIANT])
        .battlefield(2, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    attack_with(&mut engine, &[bears], P1);
    let (first, _) = division_asked_of(&engine).expect("P1 first");
    assert_eq!(first, P1);
    engine
        .apply(P1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    let (second, _) = division_asked_of(&engine).expect("then P2");
    assert_eq!(second, p2);
}
