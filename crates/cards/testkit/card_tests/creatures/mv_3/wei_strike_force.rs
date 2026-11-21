//! `cards/creatures/mv_3/wei_strike_force.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn wei_strike_force() -> CardIndex {
    card_index("002ad5fa-a20c-4d30-beef-401d89868d67")
}

/// Wei Strike Force ({2}{B} 2/1 Human Soldier): "Horsemanship"
#[test]
fn wei_strike_force_evades_ground_and_flying_blockers_but_is_blockable_by_horsemanship() {
    let mut e = lk::game(
        SEED,
        &[index::SWAMP, index::SWAMP, index::SWAMP],
        &[index::GRIZZLY_BEARS, index::WIND_DRAKE, index::SHU_CAVALRY],
        &[wei_strike_force()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, wei_strike_force());
    lk::settle(&mut e);
    let force = lk::obj(&e, lk::P0, wei_strike_force());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let drake = lk::obj(&e, lk::P1, index::WIND_DRAKE);
    let cavalry = lk::obj(&e, lk::P1, index::SHU_CAVALRY);

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let offer = lk::attack(&mut e, &[force]);
    assert!(!lk::may_block(&offer, bear, force));
    assert!(!lk::may_block(&offer, drake, force));
    assert!(lk::may_block(&offer, cavalry, force));

    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
