//! The X question of a convoke spell cast off an empty pool, with lands
//! untapped beside the creatures.
//!
//! A convoke spell whose fixed part the creatures pay is in
//! `LegalActions::castable` with nothing floating (CR 702.51a). Its X is
//! announced before anything is paid (CR 601.2b), and the lands may still
//! make mana during the cast (CR 601.2g), so while a land stands untapped
//! the question is open, as for any cast off the pool
//! (`pool_x_tests.rs`). Carbon b4, attempt 8, q158: Chord of Calling
//! (`{X}{G}{G}{G}`, convoke) beside five untapped Forests and three green
//! creatures was asked X "from 0 to 3" by the language-model seat. The
//! record replays exactly, and the engine's question there is open: the 3
//! was the seat's cap, what the Forests and Leaf Gilder's mana pay alone
//! (`baylee-seat`'s `payable_x`, `manaplan::Assist`). These tests hold the
//! engine's half: the question stays open, and X = 5 is paid.

use super::*;
use crate::engine::PlayerAction;
use crate::engine::testkit::{
    Duel, RegistryLookup, SEED, card_index, in_hand, keep_mulligans, on_battlefield,
    reach_main_phase,
};
use crate::object::Status;
use baylee_core::ids::CardIndex;

const P0: PlayerId = PlayerId::new(0);

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// `{X}{G}{G}{G}` instant, convoke: a creature card with mana value X or
/// less from the library onto the battlefield.
fn chord_of_calling() -> CardIndex {
    card_index("6789a170-f2c5-4fc0-8a45-2b2361e67410")
}

/// `{G}` 2/1 Elf, "{T}: Add {G}".
fn leaf_gilder() -> CardIndex {
    card_index("61324e37-4b79-4325-bf46-621b4270afe2")
}

/// `{1}{G}` 1/1 Elf.
fn elvish_vanguard() -> CardIndex {
    card_index("4163aa30-7e3d-424f-b003-f4a300f0071e")
}

/// `{2}{G}` 1/1 Elf.
fn fierce_empath() -> CardIndex {
    card_index("5104053a-d394-4b00-82a4-60fd9f051a6e")
}

/// Seat 0 in its first main phase with `forests` Forests and the record's
/// three green creatures, Chord of Calling in hand and nothing floating.
fn chord_board(forests: usize) -> Engine<RegistryLookup> {
    let mut board = vec![forest(); forests];
    board.extend([leaf_gilder(), elvish_vanguard(), fierce_empath()]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[chord_of_calling()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    engine
}

/// Casts Chord of Calling off the offer's `castable` list and returns the X
/// question's bounds.
#[track_caller]
fn chord_x_question(engine: &mut Engine<RegistryLookup>) -> (u32, u32) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let chord = in_hand(engine, P0, chord_of_calling()).expect("the Chord is in hand");
    assert!(
        legal.castable.contains(&chord),
        "the creatures convoke {{G}}{{G}}{{G}}, so the Chord is castable at X = 0: {legal:?}"
    );
    engine
        .apply(P0, PlayerAction::CastSpell { card: chord })
        .expect("the offer listed the cast");
    let Pending::ChooseNumber {
        player,
        min,
        max,
        reason: NumberPrompt::X,
    } = engine.pending().clone()
    else {
        panic!("X is asked first, got {:?}", engine.pending())
    };
    assert_eq!(player, P0, "the caster names X");
    (min, max)
}

/// The record: five Forests and three creatures make eight mana, so X = 5 is
/// payable (CR 107.3a, 702.51a), and with the Forests untapped the question
/// is the open one a cast off the pool asks while a source is left to tap.
#[test]
fn untapped_lands_leave_a_convoke_spells_x_open() {
    let mut engine = chord_board(5);
    let (min, max) = chord_x_question(&mut engine);
    assert_eq!(min, 0);
    assert!(
        max >= 5,
        "five Forests and three convoking creatures pay X = 5, offered 0 to {max}"
    );
}

/// With no land at all, Leaf Gilder's `{T}: Add {G}` is still a mana source
/// to tap during the cast (CR 601.2g), so the question stays open there
/// too; the payment decides what the Gilder, convoking or tapping for mana
/// but not both, pays.
#[test]
fn a_mana_creature_alone_leaves_the_x_open() {
    let mut engine = chord_board(0);
    let (min, max) = chord_x_question(&mut engine);
    assert_eq!((min, max), (0, u32::MAX));
}

/// X = 5 announced over the record's board is paid: the three creatures
/// convoke the green and the five Forests pay X in the window, and the Chord
/// stands on the stack with every Forest and creature tapped.
#[test]
fn the_record_x_of_five_is_paid() {
    let mut engine = chord_board(5);
    let (_, max) = chord_x_question(&mut engine);
    assert!(max >= 5);
    engine
        .apply(P0, PlayerAction::ChooseNumber(5))
        .expect("X = 5 is on offer");
    let creatures: Vec<ObjectId> = [leaf_gilder(), elvish_vanguard(), fierce_empath()]
        .into_iter()
        .map(|card| on_battlefield(&engine, P0, card).expect("the creature is in play"))
        .collect();
    for _ in 0..40 {
        let action = match engine.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                let objects: Vec<ObjectId> = creatures
                    .iter()
                    .copied()
                    .filter(|c| options.contains(c))
                    .collect();
                PlayerAction::ChooseTargets {
                    objects,
                    players: vec![],
                }
            }
            // The window owes X: a Forest at a time, and the pass that
            // spends the five once they float.
            Pending::Priority { legal, .. } if engine.payment_window().is_some() => legal
                .mana_abilities
                .first()
                .map_or(PlayerAction::PassPriority, |&source| {
                    PlayerAction::ActivateManaAbility { source }
                }),
            _ => break,
        };
        engine
            .apply(P0, action)
            .expect("an answer the cast offered");
    }
    let chord = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == chord_of_calling())
        });
    assert!(
        chord.is_some(),
        "the Chord was cast for X = 5: {:?}",
        engine.pending()
    );
    let untapped = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == P0 && !o.status.contains(Status::TAPPED))
        })
        .count();
    assert_eq!(untapped, 0, "eight mana paid X = 5 and the {{G}}{{G}}{{G}}");
}
