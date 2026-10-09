//! Library of Leng: "If an effect causes you to discard a card, discard it,
//! but you may put it on top of your library instead of into your
//! graveyard" (`resolve/discard.rs`, `state/discard.rs`).
//!
//! The question comes before the discard, for every door an effect's
//! discard goes through: a random discard (Mind Twist), a chosen one
//! answered by its player (Frantic Search's chain) and Balance's hands. It
//! is still a discard (CR 701.9a), and a card put on top this way is not
//! revealed (CR 701.9c). Before this, every such card went to the
//! graveyard unasked.

use super::testkit::*;
use super::*;
use crate::choice::{ArrangePlace, ArrangePrompt};
use crate::event::GameEvent;
use baylee_core::generated::index;
use baylee_core::ids::CardIndex;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn frantic_search() -> CardIndex {
    card_index("16e015b2-f8a3-4b1a-80be-58a8f5fb5e8c")
}

fn library(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .clone()
}

fn graveyard(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(seat))
        .clone()
}

fn discarded(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::Discarded { object, .. } => Some(object),
            _ => None,
        })
        .collect()
}

/// The arrangement on the table, asked of `seat`: its cards.
#[track_caller]
fn arrangement(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    let Pending::Arrange {
        player,
        cards,
        piles,
        prompt,
    } = engine.pending().clone()
    else {
        panic!(
            "expected Library of Leng's question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat);
    assert_eq!(prompt, ArrangePrompt::DiscardToLibrary);
    assert_eq!(
        piles.iter().map(|p| p.place).collect::<Vec<_>>(),
        vec![ArrangePlace::Graveyard, ArrangePlace::LibraryTop],
        "the graveyard first, so no preference is the discard as printed"
    );
    cards
}

/// p0 casts Mind Twist for 2 at p1, who holds three creatures. With
/// `leng`, p1 controls a Library of Leng.
fn twist_at_p1(seed: u64, leng: bool) -> Engine<RegistryLookup> {
    let p1_board: &[CardIndex] = if leng { &[index::LIBRARY_OF_LENG] } else { &[] };
    let mut engine = Duel::new(seed, index::PLAINS)
        .battlefield(0, &[index::SWAMP, index::SWAMP, index::SWAMP])
        .hand(0, &[index::MIND_TWIST])
        .battlefield(1, p1_board)
        .hand(
            1,
            &[index::GRIZZLY_BEARS, index::SERRA_ANGEL, index::HILL_GIANT],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, index::MIND_TWIST);
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    engine
}

/// Mind Twist's two random cards are asked about before they move; both
/// go on top in the order listed (the second listed under the first), and
/// both are still discards. Nothing reaches the graveyard.
#[test]
fn a_random_discard_may_go_on_top_of_the_library_in_an_order() {
    let mut engine = twist_at_p1(5101, true);
    let cards = arrangement(&engine, P1);
    assert_eq!(cards.len(), 2);
    for card in &cards {
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(P1))
                .contains(card),
            "asked before anything moved"
        );
    }
    let top = vec![cards[1], cards[0]];
    engine
        .apply(
            P1,
            PlayerAction::Arrange {
                piles: vec![Vec::new(), top.clone()],
            },
        )
        .unwrap();
    let lib = library(&engine, P1);
    assert_eq!(
        lib[lib.len() - 2..],
        [top[1], top[0]],
        "the first listed is the top card"
    );
    assert!(cards.iter().all(|c| !graveyard(&engine, P1).contains(c)));
    let journal = discarded(&engine);
    assert!(cards.iter().all(|c| journal.contains(c)), "still discarded");
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P1)).len(), 1);
}

/// The other answers: one card each way, and the default (everything in
/// the graveyard pile) is the discard as printed.
#[test]
fn each_discarded_card_goes_where_its_pile_says() {
    let mut engine = twist_at_p1(5102, true);
    let cards = arrangement(&engine, P1);
    engine
        .apply(
            P1,
            PlayerAction::Arrange {
                piles: vec![vec![cards[0]], vec![cards[1]]],
            },
        )
        .unwrap();
    assert!(graveyard(&engine, P1).contains(&cards[0]));
    assert_eq!(library(&engine, P1).last(), Some(&cards[1]));

    let mut engine = twist_at_p1(5102, true);
    let cards = arrangement(&engine, P1);
    engine
        .apply(
            P1,
            PlayerAction::Arrange {
                piles: vec![cards.clone(), Vec::new()],
            },
        )
        .unwrap();
    assert!(cards.iter().all(|c| graveyard(&engine, P1).contains(c)));
}

/// Without a Library of Leng the same Mind Twist asks nothing and both
/// cards are in the graveyard: the question is the card's, not the
/// discard's.
#[test]
fn without_the_library_nobody_is_asked() {
    let mut engine = twist_at_p1(5101, false);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "{:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(graveyard(&engine, P1).len(), 2);
}

/// Frantic Search: "Draw two cards, then discard two cards" — the player
/// picks the two (`DiscardChain`), and Library of Leng asks about exactly
/// those before they move. The answer comes back to the same choice, which
/// discards them without asking again.
#[test]
fn a_chosen_discard_is_asked_about_after_it_is_chosen() {
    let mut engine = Duel::new(5103, index::ISLAND)
        .battlefield(
            0,
            &[
                index::LIBRARY_OF_LENG,
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
            ],
        )
        .hand(0, &[frantic_search(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, frantic_search());
    // "Untap up to three lands": none.
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == P0),
    );
    let bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).expect("held");
    let island = in_hand(&engine, P0, index::ISLAND).expect("drawn");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears, island],
            },
        )
        .unwrap();
    let mut asked = arrangement(&engine, P0);
    asked.sort();
    let mut chosen = vec![bears, island];
    chosen.sort();
    assert_eq!(asked, chosen);
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![vec![island], vec![bears]],
            },
        )
        .unwrap();
    assert_eq!(library(&engine, P0).last(), Some(&bears));
    assert!(graveyard(&engine, P0).contains(&island));
    assert!(
        !matches!(engine.pending(), Pending::Arrange { .. }),
        "asked once"
    );
}

/// Balance: the player with more cards in hand discards down to the
/// other's count, keeping what they choose. The cards they did not keep
/// are what Library of Leng asks about.
#[test]
fn balance_asks_about_the_cards_it_discards() {
    let mut engine = Duel::new(5104, index::PLAINS)
        .battlefield(0, &[index::LIBRARY_OF_LENG, index::PLAINS, index::PLAINS])
        .hand(
            0,
            &[
                index::BALANCE,
                index::GRIZZLY_BEARS,
                index::SERRA_ANGEL,
                index::HILL_GIANT,
            ],
        )
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, index::BALANCE);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == P0),
    );
    let angel = in_hand(&engine, P0, index::SERRA_ANGEL).expect("held");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![angel],
            },
        )
        .unwrap();
    let cards = arrangement(&engine, P0);
    assert_eq!(cards.len(), 2, "the two not kept");
    assert!(!cards.contains(&angel));
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![Vec::new(), cards.clone()],
            },
        )
        .unwrap();
    let lib = library(&engine, P0);
    assert_eq!(lib[lib.len() - 2..], [cards[1], cards[0]]);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)),
        &vec![angel]
    );
}
