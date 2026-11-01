//! `cards/sorceries/mv_1/portent.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn portent() -> CardIndex {
    card_index("e744dbb6-2d56-462a-8d49-d79c73944048")
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// Portent (e744dbb6-2d56-462a-8d49-d79c73944048) — {U} Sorcery:
/// - "Look at the top three cards of target player's library, then put them back in any order.
///   You may have that player shuffle."
/// - "Draw a card at the beginning of the next turn's upkeep."
#[test]
fn portent_reorders_top_three_with_optional_shuffle_and_draws_next_upkeep() {
    // 1. Decline shuffle: chosen order of top 3 stands, and P0 draws a card at the next turn's upkeep.
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[portent()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, portent());
    engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });

    let Pending::Arrange { cards, prompt, .. } = engine.pending().clone() else {
        panic!("expected Arrange");
    };
    assert_eq!(prompt, ArrangePrompt::Order);
    let mut reversed = cards.clone();
    reversed.reverse();
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![reversed.clone()],
            },
        )
        .unwrap();

    // Decline shuffle ("No").
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let after = engine.state().zones.list(ZoneLocation::Library(P0)).clone();
    let new_top: Vec<ObjectId> = after.iter().rev().take(3).copied().collect();
    assert_eq!(
        new_top, reversed,
        "declining the shuffle preserves the chosen order of the top three cards"
    );

    // At the beginning of the next turn's upkeep (P1's upkeep, turn 2), P0 draws the top card (`reversed[0]`).
    let hand_before = hand_size(&engine, P0);
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(hand_size(&engine, P0), hand_before + 1);
    assert_eq!(
        engine.state().object(reversed[0]).unwrap().zone,
        Zone::Hand,
        "P0 drew the card placed on top of the library"
    );

    // 2. Accept shuffle ("Yes") targeting P1's library: P1's library is shuffled after ordering.
    let mut engine2 = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[portent()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, P0);

    tap_all_mana(&mut engine2, P0);
    cast_with_floating(&mut engine2, P0, portent());
    engine2.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine2, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards: cards2, .. } = engine2.pending().clone() else {
        panic!("expected Arrange");
    };
    let mut reversed2 = cards2.clone();
    reversed2.reverse();
    engine2
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![reversed2.clone()],
            },
        )
        .unwrap();
    engine2.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine2, stack_is_empty);

    let after2 = engine2
        .state()
        .zones
        .list(ZoneLocation::Library(P1))
        .clone();
    let new_top2: Vec<ObjectId> = after2.iter().rev().take(3).copied().collect();
    assert_ne!(
        new_top2, reversed2,
        "accepting the shuffle randomizes the targeted player's library"
    );
}
