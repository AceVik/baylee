//! `cards/creatures/mv_3/craven_giant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn craven_giant() -> CardIndex {
    card_index("3ebd9c8d-c840-4c2e-ace6-352a68d1c20f")
}

/// Craven Giant ({2}{R} 4/1 Giant): "This creature can't block."
#[test]
fn craven_giant_cannot_block_but_attacks_for_four() {
    let mut e = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), pearled_unicorn()],
        &[index::GRIZZLY_BEARS],
        &[craven_giant()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, craven_giant());
    lk::settle(&mut e);
    let giant = lk::obj(&e, lk::P0, craven_giant());
    let unicorn = lk::obj(&e, lk::P0, pearled_unicorn());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    reach_their_main_phase(&mut e, lk::P1);
    let offer = lk::attack(&mut e, &[bear]);
    assert!(!lk::may_block(&offer, giant, bear));
    assert!(lk::may_block(&offer, unicorn, bear));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[giant]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 16);
}
