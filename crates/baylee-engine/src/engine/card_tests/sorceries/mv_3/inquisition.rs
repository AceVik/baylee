//! `cards/sorceries/mv_3/inquisition.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn inquisition() -> CardIndex {
    card_index("9178253e-0d89-450d-a93b-ed4765e945db")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn swords_to_plowshares() -> CardIndex {
    card_index("b1544f21-7e98-461b-aed5-e748b0168c52")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

/// Seat 0 holds Inquisition (and `mine`) with {2}{B} floating; seat 1 holds
/// `theirs`.
fn board(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut hand = vec![inquisition()];
    hand.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &hand)
        .hand(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 3)]);
    engine
}

/// The hand `seat` holds right now.
fn hand_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Hand(seat)).clone()
}

/// Whether the journal shows `seat`'s hand, exactly `hand`, being revealed.
fn revealed(engine: &Engine<RegistryLookup>, seat: PlayerId, hand: &[ObjectId]) -> bool {
    engine.state().journal.entries().iter().any(|e| {
        matches!(&e.event, GameEvent::Revealed { player, cards } if *player == seat && cards == hand)
    })
}

/// Inquisition (9178253e-0d89-450d-a93b-ed4765e945db) — "Target player
/// reveals their hand. Inquisition deals damage to that player equal to the
/// number of white cards in their hand." Two white cards, a green one and a
/// land: 2 damage, and the whole hand was revealed.
#[test]
fn inquisition_reveals_the_hand_and_deals_damage_per_white_card() {
    let mut engine = board(
        &[],
        &[
            serra_angel(),
            swords_to_plowshares(),
            grizzly_bears(),
            plains(),
        ],
    );
    let theirs = hand_of(&engine, P1);
    assert_eq!(theirs.len(), 4);

    lk::announce(&mut engine, P0, inquisition(), 0, 0, &[], &[P1]);
    lk::settle(&mut engine);

    assert!(revealed(&engine, P1, &theirs), "their hand was revealed");
    assert_eq!(lk::life(&engine, P1), 18, "2 white cards, 2 damage");
    assert_eq!(lk::life(&engine, P0), 20);
    assert_eq!(hand_of(&engine, P1), theirs, "nothing is discarded");
}

/// Inquisition — no white card in the hand, no damage; the hand is still
/// revealed.
#[test]
fn inquisition_deals_no_damage_without_white_cards() {
    let mut engine = board(&[], &[grizzly_bears(), lightning_bolt(), plains()]);
    let theirs = hand_of(&engine, P1);

    lk::announce(&mut engine, P0, inquisition(), 0, 0, &[], &[P1]);
    lk::settle(&mut engine);

    assert!(revealed(&engine, P1, &theirs));
    assert_eq!(
        lk::life(&engine, P1),
        20,
        "a Plains is a land, not a white card"
    );
    assert!(in_graveyard(&engine, P0, inquisition()).is_some());
}

/// Inquisition — "target player" is any player, the caster too: it counts
/// the white cards in the caster's own hand after the spell has left it.
#[test]
fn inquisition_may_target_its_own_caster() {
    let mut engine = board(
        &[serra_angel(), swords_to_plowshares(), grizzly_bears()],
        &[],
    );
    lk::announce(&mut engine, P0, inquisition(), 0, 0, &[], &[P0]);
    let mine = hand_of(&engine, P0);
    lk::settle(&mut engine);

    assert!(revealed(&engine, P0, &mine), "my hand was revealed");
    let white = 2;
    assert_eq!(
        lk::life(&engine, P0),
        20 - white,
        "one point for each white card in my hand"
    );
    assert_eq!(lk::life(&engine, P1), 20);
}
