//! `cards/creatures/mv_2/goblin_ski_patrol.rs`, played.
//!
//! Oracle:
//! - "{1}{R}: This creature gets +2/+0 and gains flying. Its controller sacrifices it at the beginning of the next end step. Activate only once and only if you control a snow Mountain."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn goblin_ski_patrol() -> CardIndex {
    card_index("e98396d0-6952-4755-992f-813f565b9189")
}

#[test]
fn goblin_ski_patrol_requires_snow_mountain_activates_once_and_sacrifices_at_end_step() {
    let mut engine = lk::game(SEED, &[goblin_ski_patrol()], &[], &[], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 4)]);
    assert!(
        !lk::offered(&engine, goblin_ski_patrol(), P0, 0),
        "cannot activate Goblin Ski Patrol without a snow Mountain"
    );

    let mut engine = lk::game(
        SEED,
        &[goblin_ski_patrol(), index::SNOW_COVERED_MOUNTAIN],
        &[],
        &[],
        &[],
    );
    let patrol = lk::obj(&engine, P0, goblin_ski_patrol());
    lk::float(&mut engine, P0, &[(ManaColor::Red, 4)]);
    lk::use_ability(&mut engine, P0, goblin_ski_patrol(), 0, &[], &[]);
    assert_eq!(power_of(&engine, patrol), Some(3));
    assert!(keywords(&engine, patrol).contains(KeywordSet::FLYING));

    assert!(
        !lk::offered(&engine, goblin_ski_patrol(), P0, 0),
        "Goblin Ski Patrol can only be activated once"
    );

    lk::to_step(&mut engine, P0, Step::End);
    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, patrol), Zone::Graveyard);
}
