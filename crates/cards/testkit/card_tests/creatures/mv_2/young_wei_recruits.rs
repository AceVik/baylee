//! `cards/creatures/mv_2/young_wei_recruits.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn young_wei_recruits() -> CardIndex {
    card_index("cdada8f0-d001-485c-a245-5e1c1ab748ab")
}

/// Young Wei Recruits ({1}{B} 2/2 Human Soldier): "This creature can't block."
#[test]
fn young_wei_recruits_cannot_block_but_attacks_for_two() {
    let mut e = lk::game(
        SEED,
        &[index::SWAMP, index::SWAMP, pearled_unicorn()],
        &[index::GRIZZLY_BEARS],
        &[young_wei_recruits()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, young_wei_recruits());
    lk::settle(&mut e);
    let recruits = lk::obj(&e, lk::P0, young_wei_recruits());
    let unicorn = lk::obj(&e, lk::P0, pearled_unicorn());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    reach_their_main_phase(&mut e, lk::P1);
    let offer = lk::attack(&mut e, &[bear]);
    assert!(!lk::may_block(&offer, recruits, bear));
    assert!(lk::may_block(&offer, unicorn, bear));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[recruits]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
