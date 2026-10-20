//! `cards/sorceries/mv_3/icequake.rs`, played.
//!
//! Oracle:
//! - "Destroy target land. If that land was a snow land, Icequake deals 1 damage to that land's controller."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn icequake() -> CardIndex {
    card_index("8c0a6ac1-b1cc-405a-95fb-f06b7fecbb54")
}

#[test]
fn icequake_destroys_land_and_deals_one_damage_if_snow_land() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::FOREST, index::SNOW_COVERED_MOUNTAIN],
        &[icequake(), icequake()],
        &[],
    );
    let nonsnow = lk::obj(&engine, P1, index::FOREST);
    let snow = lk::obj(&engine, P1, index::SNOW_COVERED_MOUNTAIN);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
    );
    lk::cast_at(&mut engine, P0, icequake(), &[nonsnow], &[]);
    assert_eq!(lk::zone_of(&engine, nonsnow), Zone::Graveyard);
    assert_eq!(lk::life(&engine, P1), 20);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
    );
    lk::cast_at(&mut engine, P0, icequake(), &[snow], &[]);
    assert_eq!(lk::zone_of(&engine, snow), Zone::Graveyard);
    assert_eq!(lk::life(&engine, P1), 19);
}
