//! Spells that move cards between library, hand and graveyard, with real
//! cards: Divination ({2}{U}, draw two cards) and Mind Rot ({2}{B}, target
//! player discards two cards). A draw takes the top cards (CR 121.1); the
//! discarding player chooses what to discard (CR 701.9b) and discards what
//! they have when they have fewer than asked.

use super::testkit::{
    Duel, basic_forest, card_index, cast_from_hand, in_graveyard, in_hand, keep_mulligans,
    pass_until, stack_is_empty, tap_all_mana,
};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::CardIndex;

const SEED: u64 = 0x4a7d_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn divination() -> CardIndex {
    card_index("273b339c-964b-4a18-8eb5-ceb8abcdfd9e")
}

fn mind_rot() -> CardIndex {
    card_index("ad44cf74-b717-48fb-9fa2-77512024d76a")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn duel(spell: CardIndex, land: CardIndex, their_hand: usize) -> Game {
    let forest = basic_forest();
    let mut engine = Duel::new(SEED, forest)
        .hand(0, &[spell])
        .hand(1, &vec![forest; their_hand])
        .battlefield(0, &[land, forest, forest])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

fn size(engine: &Game, at: ZoneLocation) -> usize {
    engine.state().zones.list(at).len()
}

/// Casts Mind Rot at seat 1 and lets the game answer every question on
/// the way, seat 1's discard included.
fn rot(engine: &mut Game) -> Option<PlayerId> {
    let mut asked = None;
    tap_all_mana(engine, P0);
    let card = in_hand(engine, P0, mind_rot()).expect("in hand");
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    let answer = match engine.pending() {
        Pending::ChoosePlayer { .. } => PlayerAction::ChoosePlayer(P1),
        _ => PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P1],
        },
    };
    engine.apply(P0, answer).unwrap();
    // Seat 1's own discard question is answered like any other.
    for _ in 0..20 {
        if stack_is_empty(engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            return asked;
        }
        if let Pending::Priority { player, .. } = engine.pending().clone() {
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        } else {
            asked = asked.or(engine.decision_actor());
            let (player, action) = super::testkit::answer_one(engine).expect("answerable");
            engine.apply(player, action).unwrap();
        }
    }
    panic!("Mind Rot never finished: {:?}", engine.pending());
}

#[test]
fn divination_draws_the_top_two_cards() {
    let mut engine = duel(divination(), island(), 0);
    let library = engine.state().zones.list(ZoneLocation::Library(P0)).clone();
    let top: Vec<_> = library.iter().rev().take(2).copied().collect();
    let hand = size(&engine, ZoneLocation::Hand(P0));
    cast_from_hand(&mut engine, P0, divination());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(size(&engine, ZoneLocation::Hand(P0)), hand - 1 + 2);
    assert_eq!(size(&engine, ZoneLocation::Library(P0)), library.len() - 2);
    for card in top {
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(P0))
                .contains(&card)
        );
    }
    assert!(in_graveyard(&engine, P0, divination()).is_some());
}

#[test]
fn divination_draws_nothing_for_the_other_player() {
    let mut engine = duel(divination(), island(), 3);
    let theirs = size(&engine, ZoneLocation::Hand(P1));
    cast_from_hand(&mut engine, P0, divination());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(size(&engine, ZoneLocation::Hand(P1)), theirs);
}

#[test]
fn mind_rot_discards_two_of_a_bigger_hand() {
    let mut engine = duel(mind_rot(), swamp(), 4);
    let hand = size(&engine, ZoneLocation::Hand(P1));
    let yard = size(&engine, ZoneLocation::Graveyard(P1));
    let asked = rot(&mut engine);
    assert_eq!(asked, Some(P1), "the discarding player is asked to choose");
    assert_eq!(size(&engine, ZoneLocation::Hand(P1)), hand - 2);
    assert_eq!(size(&engine, ZoneLocation::Graveyard(P1)), yard + 2);
}

#[test]
fn mind_rot_against_one_card_discards_that_card() {
    let mut engine = duel(mind_rot(), swamp(), 1);
    let _ = rot(&mut engine);
    assert_eq!(size(&engine, ZoneLocation::Hand(P1)), 0);
    assert_eq!(size(&engine, ZoneLocation::Graveyard(P1)), 1);
}

#[test]
fn mind_rot_against_an_empty_hand_does_nothing_else() {
    let mut engine = duel(mind_rot(), swamp(), 0);
    let _ = rot(&mut engine);
    assert_eq!(size(&engine, ZoneLocation::Hand(P1)), 0);
    assert_eq!(size(&engine, ZoneLocation::Graveyard(P1)), 0);
    assert!(in_graveyard(&engine, P0, mind_rot()).is_some());
}

#[test]
fn mind_rot_leaves_its_casters_hand_alone() {
    let mut engine = duel(mind_rot(), swamp(), 3);
    let mine = size(&engine, ZoneLocation::Hand(P0));
    let _ = rot(&mut engine);
    assert_eq!(
        size(&engine, ZoneLocation::Hand(P0)),
        mine - 1,
        "only the cast card left seat 0's hand"
    );
}
