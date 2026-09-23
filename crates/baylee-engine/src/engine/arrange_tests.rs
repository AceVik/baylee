//! `Pending::Arrange`: the one question that puts cards into places in an
//! order, and what the engine accepts as its answer.
//!
//! Two cards raise it today, one per library end. Halimar Depths puts three
//! cards back on top in any order, and Dig Through Time puts the five it did
//! not pick on the bottom in any order. Both are read against the library
//! afterwards rather than against the answer, because a pile listed top to
//! bottom is only a claim until the library lies that way.

use super::testkit::*;
use super::*;
use crate::choice::{ArrangePile, ArrangePlace, ArrangePrompt};
use crate::zone::ZoneLocation;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 202;

fn halimar_depths() -> CardIndex {
    card_index("42d121a2-5266-483a-ab16-e0a8073cd6a3")
}

fn dig_through_time() -> CardIndex {
    card_index("f8b17b89-26ce-4208-874a-9e1d66514640")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

/// The library bottom first, top last — the order the zone stores it in.
fn library(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .clone()
}

/// Plays Halimar Depths and walks to its "put them back in any order".
fn halimar_question() -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[halimar_depths()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let card = in_hand(&engine, p0, halimar_depths()).expect("the Depths are in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("a land drop in a main phase");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    (engine, cards)
}

/// Every answer that loses, doubles or invents a card — or answers the
/// wrong number of piles — is refused, and the question still stands.
#[test]
fn an_arrangement_that_loses_doubles_or_invents_a_card_is_refused() {
    let p0 = PlayerId::new(0);
    let (mut engine, cards) = halimar_question();
    assert_eq!(cards.len(), 3, "the top three cards");
    let (a, b, c) = (cards[0], cards[1], cards[2]);
    let stranger = library(&engine, p0)[0];
    assert!(
        !cards.contains(&stranger),
        "the bottom card was not looked at"
    );

    for (answer, why) in [
        (vec![vec![a, b]], "a card left out"),
        (vec![vec![a, b, b]], "a card twice"),
        (vec![vec![a, b, stranger]], "a card that was not offered"),
        (
            vec![vec![a], vec![b, c]],
            "two piles where the question has one",
        ),
        (vec![], "no pile at all"),
    ] {
        let refused = engine.apply(p0, PlayerAction::Arrange { piles: answer });
        assert!(
            matches!(refused, Err(EngineError::IllegalAction(_))),
            "{why}: {refused:?}"
        );
        assert!(
            matches!(engine.pending(), Pending::Arrange { .. }),
            "{why}: the question still stands"
        );
    }

    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![vec![c, a, b]],
            },
        )
        .expect("every card once, in one pile");
}

/// "Put them back in any order": the first card listed is the new top card.
#[test]
fn a_top_pile_is_listed_top_to_bottom() {
    let p0 = PlayerId::new(0);
    let (mut engine, cards) = halimar_question();
    let Pending::Arrange { piles, prompt, .. } = engine.pending().clone() else {
        unreachable!("halimar_question stops at the arrangement")
    };
    assert_eq!(prompt, ArrangePrompt::Order);
    assert_eq!(
        piles,
        vec![ArrangePile::all_of(ArrangePlace::LibraryTop, 3)]
    );
    let before = library(&engine, p0);
    let offered_top_first: Vec<ObjectId> = before.iter().rev().take(3).copied().collect();
    assert_eq!(cards, offered_top_first, "offered top card first");

    let answer = vec![cards[1], cards[2], cards[0]];
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![answer.clone()],
            },
        )
        .expect("a legal order");
    pass_until(&mut engine, stack_is_empty);

    let after = library(&engine, p0);
    let top_first: Vec<ObjectId> = after.iter().rev().take(3).copied().collect();
    assert_eq!(
        top_first, answer,
        "the library now lies as the pile was listed"
    );
    assert_eq!(
        after[..after.len() - 3],
        before[..before.len() - 3],
        "and nothing under the three moved"
    );
}

/// "The rest on the bottom in any order": the last card listed is the
/// bottom card, and the picked cards are not among them.
#[test]
fn a_bottom_pile_is_listed_top_to_bottom_too() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 8])
        .hand(0, &[dig_through_time()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, dig_through_time());
    let seven: Vec<ObjectId> = library(&engine, p0).iter().rev().take(7).copied().collect();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(options.len(), 7, "the top seven");
    let picked = vec![seven[2], seven[5]];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: picked.clone(),
            },
        )
        .expect("two of the seven");

    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        panic!("the rest are ordered, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, ArrangePrompt::Order);
    assert_eq!(
        piles,
        vec![ArrangePile::all_of(ArrangePlace::LibraryBottom, 5)]
    );
    let rest: Vec<ObjectId> = seven
        .iter()
        .copied()
        .filter(|c| !picked.contains(c))
        .collect();
    assert_eq!(cards, rest, "the five not picked, in the order they lay");

    // Upside down, so an answer read the other way round would show.
    let mut answer = rest.clone();
    answer.reverse();
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![answer.clone()],
            },
        )
        .expect("a legal order");
    pass_until(&mut engine, stack_is_empty);

    let after = library(&engine, p0);
    let bottom_up: Vec<ObjectId> = after[..5].to_vec();
    let mut top_down = bottom_up;
    top_down.reverse();
    assert_eq!(
        top_down, answer,
        "the bottom five lie as the pile was listed: its last card is the bottom card"
    );
    for card in &picked {
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(p0))
                .contains(card),
            "a picked card went to the hand"
        );
    }
}
