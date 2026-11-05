//! What a real spell may target (Shock, any target), against keywords
//! written onto the creatures: hexproof hides a creature from its
//! controller's opponents only (CR 702.11b), shroud from everyone
//! (CR 702.18a), and a creature that left the battlefield is no option.

use super::testkit::{
    Duel, basic_forest, card_index, in_hand, keep_mulligans, on_battlefield, pass_until,
    quiet_creature, tap_all_mana,
};
use super::*;
use baylee_cards_dsl::KeywordSet;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0x7a46_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

fn duel() -> Game {
    let mountain = card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685");
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[shock()])
        .battlefield(0, &[mountain, quiet_creature()])
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

fn give(engine: &mut Game, id: ObjectId, keywords: KeywordSet) {
    let state = engine.dev_state_mut(P0).expect("dev board");
    state.object_mut(id).unwrap().base_mut().keywords = keywords;
    state.invalidate_projections();
    state.refresh_characteristics();
}

/// The objects Shock offers seat 0.
fn offered(engine: &mut Game) -> Vec<ObjectId> {
    tap_all_mana(engine, P0);
    let card = in_hand(engine, P0, shock()).expect("in hand");
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    match engine.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        other => panic!("a target question, got {other:?}"),
    }
}

#[test]
fn both_creatures_are_offered_without_keywords() {
    let mut engine = duel();
    let mine = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    let theirs = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    let options = offered(&mut engine);
    assert!(options.contains(&mine) && options.contains(&theirs));
}

#[test]
fn an_opponents_hexproof_creature_is_not_offered() {
    let mut engine = duel();
    let theirs = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    give(&mut engine, theirs, KeywordSet::HEXPROOF);
    assert!(!offered(&mut engine).contains(&theirs));
}

#[test]
fn my_own_hexproof_creature_is_offered_to_me() {
    let mut engine = duel();
    let mine = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    give(&mut engine, mine, KeywordSet::HEXPROOF);
    assert!(offered(&mut engine).contains(&mine));
}

#[test]
fn a_shrouded_creature_is_offered_to_nobody() {
    let mut engine = duel();
    let mine = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    give(&mut engine, mine, KeywordSet::SHROUD);
    assert!(!offered(&mut engine).contains(&mine));
}

#[test]
fn a_creature_that_left_is_no_option() {
    let mut engine = duel();
    let theirs = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .move_object(
            theirs,
            crate::zone::ZoneLocation::Graveyard(P1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    assert!(!offered(&mut engine).contains(&theirs));
}

#[test]
fn naming_a_hexproof_creature_anyway_is_refused() {
    let mut engine = duel();
    let theirs = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    give(&mut engine, theirs, KeywordSet::HEXPROOF);
    let _ = offered(&mut engine);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![theirs],
                    players: vec![],
                },
            )
            .is_err()
    );
}
