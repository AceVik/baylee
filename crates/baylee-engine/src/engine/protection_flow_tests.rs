//! Protection with real cards: Death Speakers ({W}, 1/1, protection from
//! black) against Fell ({1}{B}) and Hired Poisoner ({B}, 1/1 deathtouch).
//! Protection stops targeting, blocking by, and damage from what it names
//! (CR 702.16b, e, f) — and nothing else: Pyroclasm, which is red, still
//! deals its damage.

use super::testkit::{
    Duel, basic_forest, card_index, in_hand, keep_mulligans, on_battlefield, pass_until,
    stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, Defender, ObjectId};

const SEED: u64 = 0x9a07_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn speakers() -> CardIndex {
    card_index("4a1db129-cc7c-410a-a41e-d633e30cfdad")
}

fn poisoner() -> CardIndex {
    card_index("a7a6cdfd-10ac-49d0-a0a6-8ad37e326511")
}

fn fell() -> CardIndex {
    card_index("8f968470-ce00-43a3-95f5-2e3fe987be19")
}

fn pyroclasm() -> CardIndex {
    card_index("e4bcd4ea-e7cd-4471-8f3b-18bb51d3d70c")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn main_phase(engine: &mut Game) {
    pass_until(engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
}

fn offered_by(engine: &mut Game, spell: CardIndex) -> Vec<ObjectId> {
    tap_all_mana(engine, P0);
    let card = in_hand(engine, P0, spell).unwrap();
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    match engine.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_black_spell_cannot_target_it() {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[fell()])
        .battlefield(0, &[swamp(), swamp(), poisoner()])
        .battlefield(1, &[speakers()])
        .start();
    keep_mulligans(&mut engine);
    main_phase(&mut engine);
    let cleric = on_battlefield(&engine, P1, speakers()).unwrap();
    let rogue = on_battlefield(&engine, P0, poisoner()).unwrap();
    let options = offered_by(&mut engine, fell());
    assert!(!options.contains(&cleric));
    assert!(
        options.contains(&rogue),
        "an unprotected creature is offered"
    );
}

#[test]
fn a_red_sweeper_still_damages_it() {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[pyroclasm()])
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[speakers()])
        .start();
    keep_mulligans(&mut engine);
    main_phase(&mut engine);
    super::testkit::cast_from_hand(&mut engine, P0, pyroclasm());
    pass_until(&mut engine, stack_is_empty);
    assert!(super::testkit::in_graveyard(&engine, P1, speakers()).is_some());
    assert!(on_battlefield(&engine, P1, speakers()).is_none());
}

/// Seat 0 attacks seat 1 (who has a black Hired Poisoner) with `attacker`,
/// and the game is stopped at the point where blocks are declared.
fn attack_into_poisoner(attacker_card: CardIndex) -> (Game, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[attacker_card])
        .battlefield(1, &[poisoner()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let attacker = on_battlefield(&engine, P0, attacker_card).unwrap();
    let rogue = on_battlefield(&engine, P1, poisoner()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
            || e.state().turn.step == crate::turn::Step::DeclareBlockers
    });
    (engine, attacker, rogue)
}

/// The black Poisoner cannot block the protected cleric (CR 702.16f): the
/// block is not a legal pair and the cleric connects.
#[test]
fn a_black_creature_cannot_block_it() {
    let (mut engine, cleric, rogue) = attack_into_poisoner(speakers());
    assert!(!crate::combat::can_block(engine.state(), P1, rogue, cleric));
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 19);
    assert!(on_battlefield(&engine, P1, poisoner()).is_some());
}

/// The control: the same Poisoner may block an unprotected creature, and
/// the engine asks about it.
#[test]
fn a_black_creature_can_block_an_unprotected_one() {
    let (mut engine, elf, rogue) = attack_into_poisoner(super::testkit::quiet_creature());
    assert!(crate::combat::can_block(engine.state(), P1, rogue, elf));
    assert!(matches!(engine.pending(), Pending::ChooseBlockers { .. }));
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(rogue, elf)],
            },
        )
        .unwrap();
}

#[test]
fn damage_from_a_black_deathtoucher_is_prevented() {
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[poisoner()])
        .battlefield(1, &[speakers()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let rogue = on_battlefield(&engine, P0, poisoner()).unwrap();
    let cleric = on_battlefield(&engine, P1, speakers()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(rogue, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(cleric, rogue)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&engine, P1, speakers()).is_some(),
        "no damage, so no deathtouch"
    );
    assert!(
        on_battlefield(&engine, P0, poisoner()).is_none(),
        "the cleric's damage is not prevented"
    );
}
