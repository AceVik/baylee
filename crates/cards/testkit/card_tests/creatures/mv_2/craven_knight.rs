//! `cards/creatures/mv_2/craven_knight.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn craven_knight() -> CardIndex {
    card_index("27d138af-7c5d-4c7e-a310-63d5b03f039d")
}

/// Craven Knight ({1}{B} 2/2 Human Knight): "This creature can't block."
#[test]
fn craven_knight_cannot_block_but_attacks_for_two() {
    let mut e = lk::game(
        SEED,
        &[index::SWAMP, index::SWAMP, pearled_unicorn()],
        &[index::GRIZZLY_BEARS],
        &[craven_knight()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, craven_knight());
    lk::settle(&mut e);
    let knight = lk::obj(&e, lk::P0, craven_knight());
    let unicorn = lk::obj(&e, lk::P0, pearled_unicorn());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    reach_their_main_phase(&mut e, lk::P1);
    let offer = lk::attack(&mut e, &[bear]);
    assert!(!lk::may_block(&offer, knight, bear));
    assert!(lk::may_block(&offer, unicorn, bear));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[knight]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
