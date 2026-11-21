//! `cards/instants/mv_4/ire_of_kaminari.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "4870ec65-8067-45fb-b7ec-a6eca87c7e77"
fn ire_of_kaminari() -> CardIndex {
    card_index("4870ec65-8067-45fb-b7ec-a6eca87c7e77")
}

/// Ire of Kaminari ({3}{R}): deals damage to any target (player or creature)
/// equal to the number of Arcane cards in YOUR graveyard (ignoring non-Arcane
/// cards, opponent's Arcane cards, and the resolving copy on the stack).
#[test]
fn ire_of_kaminari_deals_damage_by_arcane_cards_in_your_graveyard() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[
            ire_of_kaminari(),
            ire_of_kaminari(),
            index::FIRST_VOLLEY,
            index::PATH_OF_ANGER_S_FLAME,
            index::LIGHTNING_BOLT,
        ],
        &[index::FIRST_VOLLEY],
    );
    hand_to_graveyard(&mut engine, P0, index::FIRST_VOLLEY);
    hand_to_graveyard(&mut engine, P0, index::PATH_OF_ANGER_S_FLAME);
    hand_to_graveyard(&mut engine, P0, index::LIGHTNING_BOLT);
    hand_to_graveyard(&mut engine, P1, index::FIRST_VOLLEY);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    // 1. 2 Arcane cards in P0's GY (ignoring Bolt and P1's First Volley) -> 2 to P1.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 8)]);
    cast_with_floating(&mut engine, P0, ire_of_kaminari());
    lk::aim(&mut engine, &[], &[P1]);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P1), 18, "2 Arcane in P0's GY -> 2 damage");

    // 2. First Ire is now in P0's GY too (3 Arcane) -> kills 2/2 Bears.
    cast_with_floating(&mut engine, P0, ire_of_kaminari());
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
