//! `cards/creatures/mv_3/wall_of_water.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Water — Defender; `{U}`: This creature gets +1/+0 until end of
/// turn. Defender keeps it off the attackers list, the pump is real and
/// wears off at cleanup.
#[test]
fn wall_of_water_pumps_for_u_until_end_of_turn_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[wall_of_water(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, wall_of_water()).expect("seated");
    assert_eq!(pt(&engine, wall), (0, 5));
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, wall_of_water(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, wall), (1, 5), "+1/+0 until end of turn");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(!attackers.contains(&wall), "Defender: it cannot attack");

    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert_eq!(pt(&engine, wall), (0, 5), "the pump ended at cleanup");
}
