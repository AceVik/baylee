//! Spells that move cards off the top of a library, with real cards: Tome
//! Scour ({U}, target player mills five), Ancestral Recall ({U}, target
//! player draws three) and Brainstorm ({U}, draw three, put two back). A
//! mill moves the top cards to the graveyard (CR 701.17a), milling more
//! than there is mills what there is and loses nobody the game, and a draw
//! for another player is that player's draw.

use super::testkit::{
    Duel, basic_forest, card_index, in_hand, keep_mulligans, pass_until, stack_is_empty,
    tap_all_mana,
};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::CardIndex;

const SEED: u64 = 0x11b4_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn tome_scour() -> CardIndex {
    card_index("446a9d7e-9c1a-4e83-8342-2ed5acae2eed")
}

fn recall() -> CardIndex {
    card_index("550c74d4-1fcb-406a-b02a-639a760a4380")
}

fn brainstorm() -> CardIndex {
    card_index("36cd2364-d113-47d1-b2c4-b088d9eb88dd")
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn duel(spell: CardIndex) -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[spell])
        .battlefield(0, &[island()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

/// Casts `spell` at player `at` and resolves it, answering every question.
fn cast_at(engine: &mut Game, spell: CardIndex, at: Option<PlayerId>) {
    tap_all_mana(engine, P0);
    let card = in_hand(engine, P0, spell).unwrap();
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    for _ in 0..30 {
        if stack_is_empty(engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChoosePlayer { .. } => {
                engine
                    .apply(P0, PlayerAction::ChoosePlayer(at.expect("a target")))
                    .unwrap();
            }
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![at.expect("a target")],
                        },
                    )
                    .unwrap();
            }
            _ => {
                let (player, action) = super::testkit::answer_one(engine).expect("answerable");
                engine.apply(player, action).unwrap();
            }
        }
    }
    panic!("never resolved: {:?}", engine.pending());
}

fn zone(engine: &Game, at: ZoneLocation) -> Vec<baylee_core::ids::ObjectId> {
    engine.state().zones.list(at).clone()
}

#[test]
fn a_mill_moves_the_top_five_to_the_graveyard() {
    let mut engine = duel(tome_scour());
    let library = zone(&engine, ZoneLocation::Library(P1));
    let top: Vec<_> = library.iter().rev().take(5).copied().collect();
    cast_at(&mut engine, tome_scour(), Some(P1));
    assert_eq!(
        zone(&engine, ZoneLocation::Library(P1)).len(),
        library.len() - 5
    );
    let yard = zone(&engine, ZoneLocation::Graveyard(P1));
    for card in top {
        assert!(yard.contains(&card));
    }
}

#[test]
fn milling_more_than_there_is_mills_what_there_is_and_loses_nobody() {
    let mut engine = duel(tome_scour());
    {
        let state = engine.dev_state_mut(P0).expect("dev board");
        let cards = state.zones.list(ZoneLocation::Library(P1)).clone();
        for card in &cards[3..] {
            state
                .move_object(
                    *card,
                    ZoneLocation::Exile(P1),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        }
    }
    assert_eq!(zone(&engine, ZoneLocation::Library(P1)).len(), 3);
    cast_at(&mut engine, tome_scour(), Some(P1));
    assert!(zone(&engine, ZoneLocation::Library(P1)).is_empty());
    assert_eq!(zone(&engine, ZoneLocation::Graveyard(P1)).len(), 3);
    assert!(
        !engine.state().players[1].has_lost(),
        "a mill is not a draw"
    );
    assert!(!engine.state().players[1].tried_empty_draw);
}

#[test]
fn recall_on_the_opponent_makes_them_draw_three() {
    let mut engine = duel(recall());
    let theirs = zone(&engine, ZoneLocation::Hand(P1)).len();
    let mine = zone(&engine, ZoneLocation::Hand(P0)).len();
    cast_at(&mut engine, recall(), Some(P1));
    assert_eq!(zone(&engine, ZoneLocation::Hand(P1)).len(), theirs + 3);
    assert_eq!(zone(&engine, ZoneLocation::Hand(P0)).len(), mine - 1);
}

#[test]
fn brainstorm_draws_three_and_puts_two_back() {
    let mut engine = duel(brainstorm());
    let hand = zone(&engine, ZoneLocation::Hand(P0)).len();
    let library = zone(&engine, ZoneLocation::Library(P0)).len();
    cast_at(&mut engine, brainstorm(), None);
    assert_eq!(
        zone(&engine, ZoneLocation::Hand(P0)).len(),
        hand - 1 + 3 - 2
    );
    assert_eq!(
        zone(&engine, ZoneLocation::Library(P0)).len(),
        library - 3 + 2
    );
}
