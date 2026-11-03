//! The cleanup step's discard (CR 514.1): the active player, and only the
//! active player, discards down to their maximum hand size, the count is
//! exactly the excess, the chosen cards go to the graveyard, and a hand at
//! the limit is asked nothing.

use super::testkit::{Duel, basic_forest, keep_mulligans, pass_until};
use super::*;
use crate::zone::ZoneLocation;

const SEED: u64 = 0x4a2d_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn duel(mine: usize, theirs: usize) -> Game {
    let forest = basic_forest();
    let mut engine = Duel::new(SEED, forest)
        .hand(0, &vec![forest; mine])
        .hand(1, &vec![forest; theirs])
        .start();
    keep_mulligans(&mut engine);
    engine
}

fn hand(engine: &Game, seat: PlayerId) -> Vec<baylee_core::ids::ObjectId> {
    engine.state().zones.list(ZoneLocation::Hand(seat)).clone()
}

/// Lets seat 0's first turn run until it is asked to discard or the turn
/// is over.
fn to_discard_or_next_turn(engine: &mut Game) {
    let turn = engine.state().turn.number;
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::DiscardChoice { .. }) || e.state().turn.number > turn
    });
}

#[test]
fn nine_cards_are_discarded_down_to_seven() {
    let mut engine = duel(9, 0);
    to_discard_or_next_turn(&mut engine);
    let over = hand(&engine, P0).len() - 7;
    assert!(
        matches!(engine.pending(), Pending::DiscardChoice { player, count }
            if *player == P0 && usize::from(*count) == over),
        "{:?}",
        engine.pending()
    );
    let chosen = hand(&engine, P0)[..over].to_vec();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: chosen.clone(),
            },
        )
        .unwrap();
    assert_eq!(hand(&engine, P0).len(), 7);
    let graveyard = engine.state().zones.list(ZoneLocation::Graveyard(P0));
    for card in chosen {
        assert!(graveyard.contains(&card), "the chosen card was discarded");
    }
}

#[test]
fn a_discard_of_the_wrong_size_is_refused() {
    let mut engine = duel(9, 0);
    to_discard_or_next_turn(&mut engine);
    let Pending::DiscardChoice { count, .. } = engine.pending().clone() else {
        panic!("a discard is asked: {:?}", engine.pending())
    };
    let cards = hand(&engine, P0);
    let too_few = cards[..usize::from(count) - 1].to_vec();
    assert!(
        engine
            .apply(P0, PlayerAction::ChooseObjects { objects: too_few })
            .is_err()
    );
    let too_many = cards[..=usize::from(count)].to_vec();
    assert!(
        engine
            .apply(P0, PlayerAction::ChooseObjects { objects: too_many })
            .is_err()
    );
    assert!(matches!(engine.pending(), Pending::DiscardChoice { .. }));
}

#[test]
fn a_card_from_the_other_hand_is_refused() {
    let mut engine = duel(8, 3);
    to_discard_or_next_turn(&mut engine);
    let Pending::DiscardChoice { count, .. } = engine.pending().clone() else {
        panic!("a discard is asked: {:?}", engine.pending())
    };
    let theirs = hand(&engine, P1)[..usize::from(count)].to_vec();
    assert!(
        engine
            .apply(P0, PlayerAction::ChooseObjects { objects: theirs })
            .is_err()
    );
}

#[test]
fn seven_cards_are_kept_without_a_question() {
    let mut engine = duel(7, 0);
    let turn = engine.state().turn.number;
    to_discard_or_next_turn(&mut engine);
    assert!(engine.state().turn.number > turn, "no discard was asked");
    assert_eq!(hand(&engine, P0).len(), 7);
}

#[test]
fn the_player_whose_turn_it_is_not_keeps_a_big_hand() {
    let mut engine = duel(0, 10);
    let turn = engine.state().turn.number;
    to_discard_or_next_turn(&mut engine);
    assert!(
        engine.state().turn.number > turn,
        "seat 1 is not asked during seat 0's cleanup"
    );
    assert!(hand(&engine, P1).len() >= 10);
}

#[test]
fn a_smaller_maximum_hand_size_discards_more() {
    let mut engine = duel(7, 0);
    engine.dev_state_mut(P0).expect("dev board").players[0].hand_modifier = -2;
    to_discard_or_next_turn(&mut engine);
    assert!(
        matches!(engine.pending(), Pending::DiscardChoice { player, count }
            if *player == P0 && *count == 2),
        "{:?}",
        engine.pending()
    );
}
