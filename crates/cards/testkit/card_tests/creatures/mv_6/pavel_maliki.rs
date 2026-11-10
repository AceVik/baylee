//! `cards/creatures/mv_6/pavel_maliki.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pavel Maliki is a 5/3 legendary creature under `Coverage::Implemented` with an activated pump ability.
/// Paying {B}{R} gives Pavel Maliki +1/+0 until end of turn.
/// Floating mana pays the exact cost, increasing projected power from 5 to 6 while leaving toughness unchanged.
/// The ability consumes the floating mana completely down to an empty pool.
#[test]
fn pavel_maliki_pumps_power_for_black_and_red_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[pavel_maliki(), swamp(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let pavel =
        on_battlefield(&engine, p0, pavel_maliki()).expect("Pavel Maliki is on battlefield");
    assert_eq!(pt(&engine, pavel), (5, 3), "printed body is 5/3");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "Swamp and Mountain provide two mana"
    );

    activate(&mut engine, p0, pavel_maliki(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, pavel),
        (6, 3),
        "Pavel Maliki gets +1/+0, becoming 6/3"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both {{B}} and {{R}} were spent"
    );
}
