//! `cards/creatures/mv_7/hellkite_igniter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hellkite_igniter() -> CardIndex {
    card_index("12aba1d4-477f-4d36-96bb-c3c7bbd51b75")
}

/// Hellkite Igniter — a 5/5 flier with haste for {5}{R}{R}: cast in the main
/// phase it is offered as an attacker at once.
#[test]
fn hellkite_igniter_costs_seven_and_can_attack_at_once() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(mountain(), 7), &[hellkite_igniter()], &[]);
    let dragon = cast_and_resolve(&mut engine, hellkite_igniter());
    assert_body(
        &engine,
        dragon,
        (5, 5),
        KeywordSet::FLYING.union(KeywordSet::HASTE),
    );
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped here")
    };
    assert!(attackers.contains(&dragon), "haste");
}

/// "{1}{R}: This creature gets +X/+0 until end of turn, where X is the number
/// of artifacts you control." With two Sol Rings (and two Mountains for the
/// price) X is 2; an opponent's artifact does not count.
#[test]
fn hellkite_igniter_pumps_by_my_artifacts() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(
        &[
            hellkite_igniter(),
            sol_ring(),
            sol_ring(),
            mountain(),
            mountain(),
        ],
        &[],
        &[sol_ring()],
    );
    let dragon = on_battlefield(&engine, p0, hellkite_igniter()).expect("out");
    assert_eq!(pt(&engine, dragon), (5, 5));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, hellkite_igniter(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dragon), (7, 5), "+2/+0 for two of my artifacts");
}
