//! `cards/creatures/mv_3/jamuraan_lion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn jamuraan_lion() -> CardIndex {
    card_index("b558541f-ae57-41f7-a5bd-e23fa35e68b4")
}

/// Jamuraan Lion ({2}{W} 3/1 Cat):
/// "{W}, {T}: Target creature can't block this turn."
#[test]
fn jamuraan_lion_stops_target_creature_from_blocking_this_turn_only() {
    let mut e = lk::game(
        SEED,
        &[jamuraan_lion(), plains(), llanowar_elves()],
        &[index::GRIZZLY_BEARS, pearled_unicorn()],
        &[],
        &[],
    );
    let elf = lk::obj(&e, lk::P0, llanowar_elves());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let unicorn = lk::obj(&e, lk::P1, pearled_unicorn());

    lk::float(&mut e, lk::P0, &[(ManaColor::White, 1)]);
    lk::use_ability(&mut e, lk::P0, jamuraan_lion(), 0, &[bear], &[]);

    let offer = lk::attack(&mut e, &[elf]);
    assert!(!lk::may_block(&offer, bear, elf));
    assert!(lk::may_block(&offer, unicorn, elf));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 19);

    // Next turn the restriction has expired.
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let offer2 = lk::attack(&mut e, &[elf]);
    assert!(lk::may_block(&offer2, bear, elf));
}
