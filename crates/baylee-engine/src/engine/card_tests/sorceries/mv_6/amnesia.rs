//! `cards/sorceries/mv_6/amnesia.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn amnesia() -> CardIndex {
    card_index("8345187a-a210-49d7-bdce-10fb2909e050")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn sol_ring_card() -> CardIndex {
    sol_ring()
}

/// Seat 0 holds Amnesia (and `mine`) with {3}{U}{U}{U} floating; seat 1
/// holds `theirs`.
fn board(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut hand = vec![amnesia()];
    hand.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &hand)
        .hand(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 6)]);
    engine
}

fn hand_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Hand(seat)).clone()
}

/// Amnesia (8345187a-a210-49d7-bdce-10fb2909e050) — "Target player reveals
/// their hand and discards all nonland cards." The land stays; the creature,
/// the instant and the artifact are discarded without being asked about, and
/// the hand was revealed first.
#[test]
fn amnesia_reveals_the_hand_and_discards_every_nonland_card() {
    let mut engine = board(
        &[],
        &[
            forest(),
            grizzly_bears(),
            lightning_bolt(),
            sol_ring_card(),
            island(),
        ],
    );
    let theirs = hand_of(&engine, P1);
    assert_eq!(theirs.len(), 5);
    let lands: Vec<ObjectId> = [forest(), island()]
        .iter()
        .map(|c| in_hand(&engine, P1, *c).expect("a land in hand"))
        .collect();

    lk::announce(&mut engine, P0, amnesia(), 0, 0, &[], &[P1]);
    lk::settle(&mut engine);

    assert!(
        engine.state().journal.entries().iter().any(|e| matches!(
            &e.event,
            GameEvent::Revealed { player, cards } if *player == P1 && cards == &theirs
        )),
        "the whole hand was revealed"
    );
    assert_eq!(hand_of(&engine, P1), lands, "only the lands remain");
    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, P1, lightning_bolt()).is_some());
    assert!(in_graveyard(&engine, P1, sol_ring_card()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P1)).len(),
        3,
        "three nonland cards discarded"
    );
}

/// Amnesia — the target's hand only: the caster's own nonland cards are
/// untouched.
#[test]
fn amnesia_leaves_the_casters_own_hand_alone() {
    let mut engine = board(&[grizzly_bears(), lightning_bolt()], &[grizzly_bears()]);
    let mine = hand_of(&engine, P0);
    lk::announce(&mut engine, P0, amnesia(), 0, 0, &[], &[P1]);
    let mine_after_cast: Vec<ObjectId> = hand_of(&engine, P0);
    lk::settle(&mut engine);

    assert_eq!(
        hand_of(&engine, P0),
        mine_after_cast,
        "my hand is as it was"
    );
    assert!(mine.len() > mine_after_cast.len(), "only the spell left it");
    assert!(in_graveyard(&engine, P1, grizzly_bears()).is_some());
    assert!(hand_of(&engine, P1).is_empty());
}

/// Amnesia — a hand of lands only loses nothing.
#[test]
fn amnesia_discards_nothing_from_a_hand_of_lands() {
    let mut engine = board(&[], &[forest(), island(), plains()]);
    let theirs = hand_of(&engine, P1);

    lk::announce(&mut engine, P0, amnesia(), 0, 0, &[], &[P1]);
    lk::settle(&mut engine);

    assert_eq!(hand_of(&engine, P1), theirs);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(P1))
            .is_empty()
    );
    assert!(in_graveyard(&engine, P0, amnesia()).is_some());
}

/// Amnesia — "target player" can be the caster: their own nonland cards go,
/// after the spell left their hand.
#[test]
fn amnesia_may_target_its_own_caster() {
    let mut engine = board(&[grizzly_bears(), forest()], &[]);
    lk::announce(&mut engine, P0, amnesia(), 0, 0, &[], &[P0]);
    lk::settle(&mut engine);

    assert!(
        in_graveyard(&engine, P0, grizzly_bears()).is_some(),
        "discarded"
    );
    assert!(in_hand(&engine, P0, forest()).is_some(), "the land is kept");
}
