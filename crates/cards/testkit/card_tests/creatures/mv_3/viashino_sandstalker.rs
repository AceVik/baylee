//! `cards/creatures/mv_3/viashino_sandstalker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn viashino_sandstalker() -> CardIndex {
    card_index("cbfccfc0-3117-4ca0-b7ce-8e2d67ec49ab")
}

/// Viashino Sandstalker ({1}{R}{R} 4/2 Lizard Warrior):
/// "Haste; At the beginning of the end step, return this creature to its
/// owner's hand."
#[test]
fn viashino_sandstalker_attacks_with_haste_and_returns_to_owners_hand_at_the_end_step() {
    let mut e = lk::game(
        SEED,
        &[mountain(), mountain(), mountain()],
        &[],
        &[viashino_sandstalker()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, viashino_sandstalker());
    lk::settle(&mut e);
    let lizard = lk::obj(&e, lk::P0, viashino_sandstalker());

    // Haste lets it attack the turn it enters.
    let _ = lk::attack(&mut e, &[lizard]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 16);
    assert!(on_battlefield(&e, lk::P0, viashino_sandstalker()).is_some());

    // Advancing to P1's turn passes P0's end step, bouncing Sandstalker to P0's hand.
    reach_their_main_phase(&mut e, lk::P1);
    assert!(on_battlefield(&e, lk::P0, viashino_sandstalker()).is_none());
    assert!(in_hand(&e, lk::P0, viashino_sandstalker()).is_some());
}
