//! `cards/creatures/mv_2/goblin_raider.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn goblin_raider() -> CardIndex {
    card_index("7f11c830-fced-40db-8c02-b3c54f3e4372")
}

/// Goblin Raider ({1}{R} 2/2 Goblin Warrior): "This creature can't block."
#[test]
fn goblin_raider_cannot_block_but_attacks_for_two() {
    let mut e = lk::game(
        SEED,
        &[mountain(), mountain(), pearled_unicorn()],
        &[index::GRIZZLY_BEARS],
        &[goblin_raider()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, goblin_raider());
    lk::settle(&mut e);
    let raider = lk::obj(&e, lk::P0, goblin_raider());
    let unicorn = lk::obj(&e, lk::P0, pearled_unicorn());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    reach_their_main_phase(&mut e, lk::P1);
    let offer = lk::attack(&mut e, &[bear]);
    assert!(!lk::may_block(&offer, raider, bear));
    assert!(lk::may_block(&offer, unicorn, bear));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[raider]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
