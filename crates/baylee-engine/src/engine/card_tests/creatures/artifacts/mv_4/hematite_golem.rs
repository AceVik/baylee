//! `cards/creatures/artifacts/mv_4/hematite_golem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Hematite Golem` prints `{{1}}{{R}}: This creature gets +2/+0 until end of turn.` on a 1/4
/// artifact creature with `Coverage::Implemented`.
/// In this scenario, two `mountain()` lands float the required mana to activate the ability.
/// Upon resolution, the layer system recomputes the golem's power from 1 to 3 while its toughness remains 4.
#[test]
fn hematite_golem_pumps_power_by_two_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), hematite_golem()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem = on_battlefield(&engine, p0, hematite_golem()).expect("golem is seated");
    assert_eq!(pt(&engine, golem), (1, 4));

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    activate(&mut engine, p0, hematite_golem(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, golem),
        (3, 4),
        "`Hematite Golem` should now be a 3/4 until end of turn"
    );
}
