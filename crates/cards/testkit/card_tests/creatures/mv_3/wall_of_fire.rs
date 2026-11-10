//! `cards/creatures/mv_3/wall_of_fire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Fire — Defender; `{R}`: This creature gets +1/+0 until end of
/// turn.
#[test]
fn wall_of_fire_pumps_for_r_until_end_of_turn_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[wall_of_fire(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, wall_of_fire()).expect("seated");
    assert_eq!(pt(&engine, wall), (0, 5));
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, wall_of_fire(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, wall), (1, 5), "+1/+0 until end of turn");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(!attackers.contains(&wall), "Defender: it cannot attack");
}

/// Wall of Fire: "{R}: This creature gets +1/+0 until end of turn." Two
/// activations stack, last through the turn, and are gone when the next
/// turn begins.
#[test]
fn wall_of_fire_pump_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[wall_of_fire(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, wall_of_fire()).expect("seated");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, wall_of_fire(), 0);
    activate(&mut engine, p0, wall_of_fire(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, wall), (2, 5), "two activations stack");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && e.state().turn.active == p0
    });
    assert_eq!(pt(&engine, wall), (2, 5), "and last past combat");
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, wall),
        (0, 5),
        "the pump is gone after the cleanup step"
    );
}
