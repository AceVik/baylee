//! `cards/creatures/mv_2/wei_scout.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn wei_scout() -> CardIndex {
    card_index("f7fa054f-39cb-47d3-a55d-3f70ae164063")
}

/// Wei Scout ({1}{B} 1/1 Human Soldier Scout): "Horsemanship"
#[test]
fn wei_scout_evades_ground_and_flying_blockers_but_is_blockable_by_horsemanship() {
    let mut e = lk::game(
        SEED,
        &[index::SWAMP, index::SWAMP],
        &[index::GRIZZLY_BEARS, index::WIND_DRAKE, index::SHU_CAVALRY],
        &[wei_scout()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, wei_scout());
    lk::settle(&mut e);
    let scout = lk::obj(&e, lk::P0, wei_scout());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let drake = lk::obj(&e, lk::P1, index::WIND_DRAKE);
    let cavalry = lk::obj(&e, lk::P1, index::SHU_CAVALRY);

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    let offer = lk::attack(&mut e, &[scout]);
    assert!(!lk::may_block(&offer, bear, scout));
    assert!(!lk::may_block(&offer, drake, scout));
    assert!(lk::may_block(&offer, cavalry, scout));

    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 19);
}
