//! `cards/creatures/mv_2/hulking_goblin.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn hulking_goblin() -> CardIndex {
    card_index("1451b623-8214-4d81-9436-62dbe8b79539")
}

/// Hulking Goblin ({1}{R} 2/2 Goblin): "This creature can't block."
#[test]
fn hulking_goblin_cannot_block_but_attacks_for_two() {
    let mut e = lk::game(
        SEED,
        &[mountain(), mountain(), pearled_unicorn()],
        &[index::GRIZZLY_BEARS],
        &[hulking_goblin()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, hulking_goblin());
    lk::settle(&mut e);
    let goblin = lk::obj(&e, lk::P0, hulking_goblin());
    let unicorn = lk::obj(&e, lk::P0, pearled_unicorn());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    reach_their_main_phase(&mut e, lk::P1);
    let offer = lk::attack(&mut e, &[bear]);
    assert!(!lk::may_block(&offer, goblin, bear));
    assert!(lk::may_block(&offer, unicorn, bear));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[goblin]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
