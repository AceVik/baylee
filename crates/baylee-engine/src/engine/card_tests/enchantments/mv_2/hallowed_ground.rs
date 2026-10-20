//! `cards/enchantments/mv_2/hallowed_ground.rs`, played.
//!
//! Oracle:
//! - "{W}{W}: Return target nonsnow land you control to its owner's hand."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn hallowed_ground() -> CardIndex {
    card_index("ee8fd844-4551-4e44-9740-00d2e76eb337")
}

#[test]
fn hallowed_ground_returns_nonsnow_land_to_hand_and_rejects_snow_land() {
    let mut engine = lk::game(
        SEED,
        &[
            hallowed_ground(),
            index::FOREST,
            index::SNOW_COVERED_MOUNTAIN,
        ],
        &[],
        &[],
        &[],
    );
    let nonsnow = lk::obj(&engine, P0, index::FOREST);
    let snow = lk::obj(&engine, P0, index::SNOW_COVERED_MOUNTAIN);
    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);

    activate(&mut engine, P0, hallowed_ground(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(options.contains(&nonsnow));
    assert!(!options.contains(&snow));
    lk::aim(&mut engine, &[nonsnow], &[]);
    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, nonsnow), Zone::Hand);
}
