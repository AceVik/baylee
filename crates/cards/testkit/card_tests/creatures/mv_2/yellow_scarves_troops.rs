//! `cards/creatures/mv_2/yellow_scarves_troops.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn yellow_scarves_troops() -> CardIndex {
    card_index("fcfa5671-a7b1-4170-9c72-f18a9eccfa31")
}

/// Yellow Scarves Troops ({1}{R} 2/2 Human Soldier): "This creature can't block."
#[test]
fn yellow_scarves_troops_cannot_block_but_attacks_for_two() {
    let mut e = lk::game(
        SEED,
        &[mountain(), mountain(), pearled_unicorn()],
        &[index::GRIZZLY_BEARS],
        &[yellow_scarves_troops()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, yellow_scarves_troops());
    lk::settle(&mut e);
    let troops = lk::obj(&e, lk::P0, yellow_scarves_troops());
    let unicorn = lk::obj(&e, lk::P0, pearled_unicorn());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    reach_their_main_phase(&mut e, lk::P1);
    let offer = lk::attack(&mut e, &[bear]);
    assert!(!lk::may_block(&offer, troops, bear));
    assert!(lk::may_block(&offer, unicorn, bear));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[troops]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
