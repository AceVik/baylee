//! Removal with real cards: Fell ({1}{B} sorcery, destroy target
//! creature) and Unsummon ({U} instant, return target creature to its
//! owner's hand). A destroyed creature goes to its owner's graveyard
//! (CR 701.8a), indestructible stops it (CR 702.12b), a returned creature
//! is a new object in its owner's hand (CR 400.7), and a sorcery keeps to
//! sorcery timing (CR 307.1).

use super::testkit::{
    Duel, basic_forest, card_index, in_graveyard, in_hand, keep_mulligans, on_battlefield,
    pass_until, quiet_creature, stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0x4e30_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

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

fn duel() -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[fell(), unsummon()])
        .battlefield(0, &[swamp(), swamp(), island()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

fn cast_at(engine: &mut Game, card: CardIndex, target: ObjectId) -> bool {
    tap_all_mana(engine, P0);
    let spell = in_hand(engine, P0, card).expect("in hand");
    if engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .is_err()
    {
        return false;
    }
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    while !matches!(engine.pending(), Pending::Priority { .. }) {
        let (player, action) = super::testkit::answer_one(engine).expect("answerable");
        engine.apply(player, action).unwrap();
    }
    pass_until(engine, stack_is_empty);
    true
}

fn elf(engine: &Game) -> ObjectId {
    on_battlefield(engine, P1, quiet_creature()).expect("seat 1's elf")
}

#[test]
fn a_destroyed_creature_goes_to_its_owners_graveyard() {
    let mut engine = duel();
    let target = elf(&engine);
    assert!(cast_at(&mut engine, fell(), target));
    assert!(on_battlefield(&engine, P1, quiet_creature()).is_none());
    assert!(in_graveyard(&engine, P1, quiet_creature()).is_some());
    assert!(in_graveyard(&engine, P0, quiet_creature()).is_none());
    assert!(in_graveyard(&engine, P0, fell()).is_some());
}

#[test]
fn an_indestructible_creature_survives_destroy() {
    let mut engine = duel();
    let target = elf(&engine);
    {
        let state = engine.dev_state_mut(P0).expect("dev board");
        state.object_mut(target).unwrap().base_mut().keywords =
            baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE;
        state.invalidate_projections();
    }
    assert!(cast_at(&mut engine, fell(), target));
    assert!(on_battlefield(&engine, P1, quiet_creature()).is_some());
    assert!(
        in_graveyard(&engine, P0, fell()).is_some(),
        "the spell resolved"
    );
}

#[test]
fn a_bounced_creature_returns_to_its_owners_hand() {
    let mut engine = duel();
    let target = elf(&engine);
    let hand = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(P1))
        .len();
    assert!(cast_at(&mut engine, unsummon(), target));
    assert!(on_battlefield(&engine, P1, quiet_creature()).is_none());
    assert!(in_hand(&engine, P1, quiet_creature()).is_some());
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(P1))
            .len(),
        hand + 1
    );
}

#[test]
fn an_indestructible_creature_can_still_be_bounced() {
    let mut engine = duel();
    let target = elf(&engine);
    {
        let state = engine.dev_state_mut(P0).expect("dev board");
        state.object_mut(target).unwrap().base_mut().keywords =
            baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE;
        state.invalidate_projections();
    }
    assert!(cast_at(&mut engine, unsummon(), target));
    assert!(in_hand(&engine, P1, quiet_creature()).is_some());
}

#[test]
fn a_bounced_creature_loses_its_damage_and_counters() {
    let mut engine = duel();
    let target = elf(&engine);
    {
        let state = engine.dev_state_mut(P0).expect("dev board");
        let obj = state.object_mut(target).unwrap();
        obj.counters.set(crate::object::CounterKind::P1P1, 2);
    }
    assert!(cast_at(&mut engine, unsummon(), target));
    let card = in_hand(&engine, P1, quiet_creature()).unwrap();
    let obj = engine.state().object(card).unwrap();
    assert!(obj.counters.is_empty());
    assert_eq!(obj.damage, 0);
}

#[test]
fn a_sorcery_is_refused_in_the_opponents_turn() {
    let mut engine = duel();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let spell = in_hand(&engine, P0, fell()).unwrap();
    tap_all_mana(&mut engine, P0);
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: spell })
            .is_err()
    );
}

#[test]
fn an_instant_bounce_works_in_the_opponents_turn() {
    let mut engine = duel();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let target = elf(&engine);
    assert!(cast_at(&mut engine, unsummon(), target));
    assert!(in_hand(&engine, P1, quiet_creature()).is_some());
}
