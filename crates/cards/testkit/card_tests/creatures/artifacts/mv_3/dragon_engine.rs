//! `cards/creatures/artifacts/mv_3/dragon_engine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dragon Engine` prints `{{2}}: This creature gets +1/+0 until end of turn.` on a 1/3
/// artifact creature with `Coverage::Implemented`.
/// In this scenario, two `forest()` lands provide the mana to activate its pump ability.
/// Upon resolution, the layer system recomputes its characteristics, raising its power
/// from 1 to 2 while its toughness remains 3.
#[test]
fn dragon_engine_pumps_its_power_by_one_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), dragon_engine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let deng = on_battlefield(&engine, p0, dragon_engine()).expect("dragon engine is seated");
    assert_eq!(pt(&engine, deng), (1, 3));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, dragon_engine(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, deng),
        (2, 3),
        "`Dragon Engine` should now be a 2/3 until end of turn"
    );
}
