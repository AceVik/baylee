//! `cards/creatures/mv_1/jungle_lion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn jungle_lion() -> CardIndex {
    card_index("45b42694-9ee8-44f3-9de3-aa2c0f1b6a13")
}

/// Jungle Lion ({G} 2/1 Cat): "This creature can't block."
#[test]
fn jungle_lion_cannot_block_but_attacks_for_two() {
    let mut e = lk::game(
        SEED,
        &[forest(), pearled_unicorn()],
        &[index::GRIZZLY_BEARS],
        &[jungle_lion()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, jungle_lion());
    lk::settle(&mut e);
    let lion = lk::obj(&e, lk::P0, jungle_lion());
    let unicorn = lk::obj(&e, lk::P0, pearled_unicorn());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);

    reach_their_main_phase(&mut e, lk::P1);
    let offer = lk::attack(&mut e, &[bear]);
    assert!(!lk::may_block(&offer, lion, bear));
    assert!(lk::may_block(&offer, unicorn, bear));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[lion]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
