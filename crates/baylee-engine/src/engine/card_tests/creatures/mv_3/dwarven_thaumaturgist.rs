//! `cards/creatures/mv_3/dwarven_thaumaturgist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::DWARVEN_THAUMATURGIST;

/// `Dwarven Thaumaturgist` ({2}{R}, Creature — Dwarf Shaman 1/2) taps to
/// switch target creature's power and toughness until end of turn.
#[test]
fn dwarven_thaumaturgist_switches_target_creature_power_and_toughness() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[DWARVEN_THAUMATURGIST])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shaman = on_battlefield(&engine, p0, DWARVEN_THAUMATURGIST).unwrap();
    assert_eq!(pt(&engine, shaman), (1, 2));

    activate(&mut engine, p0, DWARVEN_THAUMATURGIST, 0);
    aim_at(&mut engine, p0, shaman);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, shaman), (2, 1));
}
