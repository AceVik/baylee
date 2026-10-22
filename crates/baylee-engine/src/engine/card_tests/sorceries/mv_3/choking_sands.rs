//! `cards/sorceries/mv_3/choking_sands.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CHOKING_SANDS, CITY_OF_BRASS};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Choking Sands (3bcb7436-f7d9-4bb0-81be-7a3db9d8329f) — {1}{B}{B} Sorcery:
/// "Destroy target non-Swamp land. If that land was nonbasic, Choking Sands
/// deals 2 damage to the land's controller."
#[test]
fn choking_sands_destroys_non_swamp_and_damages_controller_only_if_nonbasic() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[CITY_OF_BRASS, forest()])
        .hand(0, &[CHOKING_SANDS, CHOKING_SANDS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let nonbasic = on_battlefield(&engine, P1, CITY_OF_BRASS).unwrap();
    let basic = on_battlefield(&engine, P1, forest()).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, CHOKING_SANDS);
    aim_at(&mut engine, P0, nonbasic);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, CITY_OF_BRASS).is_none());
    assert_eq!(engine.state().players[1].life, 18);

    // Second cast on basic Forest destroys it without dealing 2 damage.
    cast_with_floating(&mut engine, P0, CHOKING_SANDS);
    aim_at(&mut engine, P0, basic);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, forest()).is_none());
    assert_eq!(engine.state().players[1].life, 18);
}
