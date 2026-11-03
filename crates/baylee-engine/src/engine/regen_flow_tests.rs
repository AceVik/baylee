//! Regeneration with real cards (Cudgel Troll, {G}: regenerate; Fell,
//! destroy target creature): a shield replaces the next destruction this
//! turn by tapping the creature and removing all damage from it
//! (CR 701.19a); one shield answers one destruction, and an unused shield
//! is gone at the end of the turn.

use super::testkit::{
    Duel, basic_forest, card_index, in_graveyard, in_hand, keep_mulligans, on_battlefield,
    pass_until, stack_is_empty, tap_mana_where,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0x2e6e_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn troll() -> CardIndex {
    card_index("c2f86723-4765-4dac-a9f6-9cfa543613b2")
}

fn fell() -> CardIndex {
    card_index("8f968470-ce00-43a3-95f5-2e3fe987be19")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

/// Seat 1 owns a troll and a Forest; seat 0 has Fell and the mana for it.
fn duel() -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[fell(), fell()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[troll(), basic_forest(), basic_forest()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

fn the_troll(engine: &Game) -> Option<ObjectId> {
    on_battlefield(engine, P1, troll())
}

/// Seat 0 casts Fell at the troll; seat 1, given priority with Fell on the
/// stack, regenerates `shields` times first.
fn fell_the_troll(engine: &mut Game, shields: usize) {
    let target = the_troll(engine).expect("the troll");
    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == P1 && o.card.is_some_and(|c| c.index == basic_forest())
            })
        })
        .collect();
    let swamps: Vec<ObjectId> = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == P0
                    && o.card.is_some_and(|c| c.index == swamp())
                    && !o.status.contains(crate::object::Status::TAPPED)
            })
        })
        .take(2)
        .collect();
    tap_mana_where(engine, P0, |id| swamps.contains(&id));
    let card = in_hand(engine, P0, fell()).expect("Fell in hand");
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == P1));
    if shields > 0 {
        tap_mana_where(engine, P1, |id| forests.contains(&id));
        for _ in 0..shields {
            engine
                .apply(
                    P1,
                    PlayerAction::ActivateAbility {
                        source: target,
                        ability_index: 0,
                    },
                )
                .unwrap();
            pass_until(engine, |e| {
                e.state().zones.list(crate::zone::ZoneLocation::Stack).len() == 1
                    && matches!(e.pending(), Pending::Priority { .. })
            });
        }
    }
    pass_until(engine, stack_is_empty);
}

#[test]
fn without_a_shield_the_troll_is_destroyed() {
    let mut engine = duel();
    fell_the_troll(&mut engine, 0);
    assert!(the_troll(&engine).is_none());
    assert!(in_graveyard(&engine, P1, troll()).is_some());
}

#[test]
fn a_shield_keeps_the_troll_and_taps_it() {
    let mut engine = duel();
    fell_the_troll(&mut engine, 1);
    let troll = the_troll(&engine).expect("regenerated");
    assert!(
        engine
            .state()
            .object(troll)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
}

#[test]
fn a_shield_removes_the_damage_on_the_troll() {
    let mut engine = duel();
    let troll = the_troll(&engine).unwrap();
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .object_mut(troll)
        .unwrap()
        .damage = 2;
    fell_the_troll(&mut engine, 1);
    assert_eq!(engine.state().object(troll).unwrap().damage, 0);
}

#[test]
fn one_shield_answers_one_destruction() {
    let mut engine = duel();
    fell_the_troll(&mut engine, 1);
    assert!(the_troll(&engine).is_some());
    fell_the_troll(&mut engine, 0);
    assert!(
        the_troll(&engine).is_none(),
        "the second Fell finds no shield"
    );
}

#[test]
fn an_unused_shield_ends_with_the_turn() {
    let mut engine = duel();
    let troll = the_troll(&engine).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    tap_mana_where(&mut engine, P1, |_| true);
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: troll,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(troll).unwrap().regeneration_shields,
        1
    );
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    let regen_left = engine
        .state()
        .object(troll)
        .map_or(0, |o| o.regeneration_shields);
    assert_eq!(regen_left, 0);
}
