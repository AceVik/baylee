//! `cards/creatures/mv_3/shu_cavalry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn shu_cavalry() -> CardIndex {
    card_index("d9180f63-c745-41a1-9f01-273e7ee13176")
}

/// Shu Cavalry ({2}{W} 2/2 Human Soldier): "Horsemanship"
#[test]
fn shu_cavalry_evades_ground_and_flying_blockers_but_is_blockable_by_horsemanship() {
    let mut e = lk::game(
        SEED,
        &[plains(), plains(), plains()],
        &[index::GRIZZLY_BEARS, index::WIND_DRAKE, index::WEI_SCOUT],
        &[shu_cavalry()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, shu_cavalry());
    lk::settle(&mut e);
    let cavalry = lk::obj(&e, lk::P0, shu_cavalry());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let drake = lk::obj(&e, lk::P1, index::WIND_DRAKE);
    let scout = lk::obj(&e, lk::P1, index::WEI_SCOUT);

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let offer = lk::attack(&mut e, &[cavalry]);
    assert!(!lk::may_block(&offer, bear, cavalry));
    assert!(!lk::may_block(&offer, drake, cavalry));
    assert!(lk::may_block(&offer, scout, cavalry));

    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
