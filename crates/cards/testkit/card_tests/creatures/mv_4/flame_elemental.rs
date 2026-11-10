//! `cards/creatures/mv_4/flame_elemental.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flame_elemental() -> CardIndex {
    card_index("975226a4-2d03-4993-9337-cfd6595011bd")
}

/// Flame Elemental (975226a4-2d03-4993-9337-cfd6595011bd) — {2}{R}{R} 3/2:
/// "{R}, {T}, Sacrifice this creature: It deals damage equal to its power to
/// target creature."
#[test]
fn flame_elemental_sacrifices_to_deal_its_power_in_damage_to_target_creature() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[flame_elemental(), mountain()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, flame_elemental(), 0);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, flame_elemental()).is_none());
    assert!(on_battlefield(&engine, P1, HILL_GIANT).is_none());
}
