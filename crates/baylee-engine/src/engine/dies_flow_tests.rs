//! "When this creature dies" with real cards: Tarpan (dies: you gain 1
//! life) and Feral Prowler (dies: draw a card). Dying is being put into a
//! graveyard from the battlefield (CR 700.4), so a bounce does not trigger
//! it; the trigger is its controller's, whoever killed it (CR 603.3a).

use super::testkit::{
    Duel, basic_forest, card_index, in_hand, keep_mulligans, on_battlefield, pass_until,
    stack_is_empty, tap_all_mana,
};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0xd1e5_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn tarpan() -> CardIndex {
    card_index("b1e1ebc8-cd09-4280-a169-58ab7ece43ff")
}

fn prowler() -> CardIndex {
    card_index("5bebba2d-867f-4ed1-a308-904fac32c8b6")
}

fn fell() -> CardIndex {
    card_index("8f968470-ce00-43a3-95f5-2e3fe987be19")
}

fn unsummon() -> CardIndex {
    card_index("837182db-1bf3-4a2c-bd01-1af9d9873561")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn duel(spell: CardIndex, lands: &[CardIndex], mine: &[CardIndex], theirs: &[CardIndex]) -> Game {
    let mut mine_all = lands.to_vec();
    mine_all.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[spell])
        .battlefield(0, &mine_all)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

fn cast_at(engine: &mut Game, spell: CardIndex, target: ObjectId) {
    tap_all_mana(engine, P0);
    let card = in_hand(engine, P0, spell).expect("in hand");
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });
}

fn life(engine: &Game, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

#[test]
fn a_destroyed_tarpan_gains_its_controller_a_life() {
    let mut engine = duel(fell(), &[swamp(), swamp()], &[tarpan()], &[]);
    let horse = on_battlefield(&engine, P0, tarpan()).unwrap();
    cast_at(&mut engine, fell(), horse);
    assert!(on_battlefield(&engine, P0, tarpan()).is_none());
    assert_eq!(life(&engine, P0), 21);
}

#[test]
fn the_trigger_belongs_to_the_creatures_controller_not_its_killer() {
    let mut engine = duel(fell(), &[swamp(), swamp()], &[], &[tarpan()]);
    let horse = on_battlefield(&engine, P1, tarpan()).unwrap();
    cast_at(&mut engine, fell(), horse);
    assert_eq!(life(&engine, P1), 21);
    assert_eq!(life(&engine, P0), 20);
}

#[test]
fn a_bounced_tarpan_did_not_die() {
    let mut engine = duel(unsummon(), &[island()], &[tarpan()], &[]);
    let horse = on_battlefield(&engine, P0, tarpan()).unwrap();
    cast_at(&mut engine, unsummon(), horse);
    assert!(in_hand(&engine, P0, tarpan()).is_some());
    assert_eq!(life(&engine, P0), 20);
}

#[test]
fn a_dying_prowler_draws_its_controller_a_card() {
    let mut engine = duel(fell(), &[swamp(), swamp()], &[prowler()], &[]);
    let cat = on_battlefield(&engine, P0, prowler()).unwrap();
    let library = engine.state().zones.list(ZoneLocation::Library(P0)).len();
    cast_at(&mut engine, fell(), cat);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P0)).len(),
        library - 1
    );
}

#[test]
fn a_tarpan_dying_in_combat_triggers_too() {
    let troll = card_index("c2f86723-4765-4dac-a9f6-9cfa543613b2");
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[troll])
        .battlefield(1, &[tarpan()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let a = on_battlefield(&engine, P0, troll).unwrap();
    let b = on_battlefield(&engine, P1, tarpan()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(a, baylee_core::ids::Defender::Player(P1))],
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
                blockers: vec![(b, a)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(on_battlefield(&engine, P1, tarpan()).is_none());
    assert_eq!(life(&engine, P1), 21);
}
