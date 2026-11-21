//! `cards/creatures/mv_2/wu_light_cavalry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn wu_light_cavalry() -> CardIndex {
    card_index("c24da532-311b-4b83-82d2-7dc80409ba12")
}

/// Wu Light Cavalry ({1}{U} 1/2 Human Soldier): "Horsemanship"
#[test]
fn wu_light_cavalry_attacks_and_blocks_with_horsemanship() {
    let mut e = lk::game(
        SEED,
        &[island(), island()],
        &[index::GRIZZLY_BEARS, index::WIND_DRAKE, index::WEI_SCOUT],
        &[wu_light_cavalry()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, wu_light_cavalry());
    lk::settle(&mut e);
    let wu = lk::obj(&e, lk::P0, wu_light_cavalry());
    let bear = lk::obj(&e, lk::P1, index::GRIZZLY_BEARS);
    let drake = lk::obj(&e, lk::P1, index::WIND_DRAKE);
    let scout = lk::obj(&e, lk::P1, index::WEI_SCOUT);

    // Wu Light Cavalry (1/2 horsemanship) can block P1's Wei Scout (1/1 horsemanship) and survive.
    reach_their_main_phase(&mut e, lk::P1);
    let offer_def = lk::attack(&mut e, &[scout]);
    assert!(lk::may_block(&offer_def, wu, scout));
    lk::block(&mut e, &[(wu, scout)]);
    lk::through_combat(&mut e);
    assert!(on_battlefield(&e, lk::P0, wu_light_cavalry()).is_some());
    assert!(on_battlefield(&e, lk::P1, index::WEI_SCOUT).is_none());

    reach_their_main_phase(&mut e, lk::P0);
    let offer_atk = lk::attack(&mut e, &[wu]);
    assert!(!lk::may_block(&offer_atk, bear, wu));
    assert!(!lk::may_block(&offer_atk, drake, wu));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 19);
}
