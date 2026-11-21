//! `cards/creatures/mv_3/kavu_aggressor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn kavu_aggressor() -> CardIndex {
    card_index("84c00644-d0e2-418b-b931-4e972ecb3366")
}

/// Kavu Aggressor ({2}{R} 3/2 Kavu):
/// "Kicker {4}; This creature can't block. If this creature was kicked, it
/// enters with a +1/+1 counter on it."
#[test]
fn kavu_aggressor_cannot_block_and_enters_four_three_when_kicked() {
    let mut e_base = lk::game(SEED, &[], &[], &[kavu_aggressor()], &[]);
    lk::float(&mut e_base, lk::P0, &[(ManaColor::Red, 7)]);
    cast_with_floating(&mut e_base, lk::P0, kavu_aggressor());
    lk::yes_no(&mut e_base, false);
    lk::settle(&mut e_base);
    assert_eq!(
        pt(&e_base, lk::obj(&e_base, lk::P0, kavu_aggressor())),
        (3, 2)
    );

    let mut e = lk::game(
        SEED,
        &[pearled_unicorn()],
        &[index::GRIZZLY_BEARS],
        &[kavu_aggressor()],
        &[],
    );
    let unicorn = lk::obj(&e, lk::P0, pearled_unicorn());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 7)]);
    cast_with_floating(&mut e, lk::P0, kavu_aggressor());
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    let kavu = lk::obj(&e, lk::P0, kavu_aggressor());
    assert_eq!(counters_on(&e, kavu, CounterKind::P1P1), 1);
    assert_eq!(pt(&e, kavu), (4, 3));

    reach_their_main_phase(&mut e, lk::P1);
    let offer = lk::attack(&mut e, &[bear]);
    assert!(!lk::may_block(&offer, kavu, bear));
    assert!(lk::may_block(&offer, unicorn, bear));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    reach_their_main_phase(&mut e, lk::P0);
    let _ = lk::attack(&mut e, &[kavu]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 16);
}
