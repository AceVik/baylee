//! `cards/artifacts/mv_3/chariot_of_the_sun.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CHARIOT_OF_THE_SUN, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);

/// Chariot of the Sun (b0af249d-9255-42f5-bafc-ac61ee281930) — {3} Artifact:
/// "{2}, {T}: Until end of turn, target creature you control gains flying and
/// has base toughness 1."
#[test]
fn chariot_of_the_sun_grants_flying_and_sets_base_toughness_to_one() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[CHARIOT_OF_THE_SUN, plains(), plains(), HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, CHARIOT_OF_THE_SUN, 0);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, giant).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, giant), (3, 1));
}
