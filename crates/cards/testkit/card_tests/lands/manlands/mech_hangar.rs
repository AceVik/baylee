//! `cards/lands/manlands/mech_hangar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mech Hangar prints three lines, and the third is the one a turn reads:
/// "{3}, {T}: Target Vehicle becomes an artifact creature until end of turn."
/// Two Smuggler's Copters stand side by side — Vehicles that are artifacts
/// and nothing else — so the animation is the only thing that can tell them
/// apart, and the counter-half of "target" is the copy nobody names. The {3}
/// is paid out of three Forests while the Hangar is left standing, because
/// its own {T} is the rest of the cost (CR 601.2c names the target before
/// CR 601.2h pays). The offer is the last reader: the Copter that just became
/// a 3/3 flier is the one the combat step now lists.
#[test]
#[allow(clippy::too_many_lines)] // one activation, read from the pool, the type line and the attack offer
fn mech_hangar_animates_the_vehicle_it_targets_and_no_other() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                smugglers_copter(),
                smugglers_copter(),
            ],
        )
        .hand(0, &[mech_hangar()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The land under test is *played*, not seeded onto the board.
    let hangar = play_land(&mut engine, p0, mech_hangar());

    let copters = all_on_battlefield(&engine, p0, smugglers_copter());
    assert_eq!(
        copters.len(),
        2,
        "two Vehicles, one of which stays a noncreature artifact"
    );
    let (target, bystander) = (copters[0], copters[1]);
    let before = types(&engine, target);
    assert!(
        before.contains(TypeSet::ARTIFACT) && !before.contains(TypeSet::CREATURE),
        "a Vehicle is an artifact and nothing else until something animates it \
         (CR 301.7): {before:?}"
    );
    assert_eq!(
        pt(&engine, target),
        (3, 3),
        "the body the card prints is already there"
    );

    // The {3} first, and the Hangar deliberately kept back: its {T} is half
    // the cost, and a source tapped to pay for the ability would have been
    // spent before the ability could spend it.
    tap_all_mana_but(&mut engine, p0, Some(mech_hangar()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, three mana"
    );
    assert!(
        !is_tapped(&engine, hangar),
        "and the Hangar is untouched by the tapping"
    );

    // Ability 2: the {C} and the any-color mana ability are printed ahead of it.
    activate(&mut engine, p0, mech_hangar(), 2);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the animation targets a Vehicle, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&target) && options.contains(&bystander),
        "\"target Vehicle\" reaches either of the Copters: {options:?}"
    );
    assert!(
        !options.contains(&hangar),
        "the permanent paying is a land and no Vehicle: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the target is named before the cost is paid (CR 601.2c, then 601.2h)"
    );
    assert!(
        !is_tapped(&engine, hangar),
        "so the {{T}} in the cost has not been paid either"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}} comes out of the pool with the rest of the cost"
    );
    assert!(is_tapped(&engine, hangar), "along with the {{T}}");

    pass_until(&mut engine, |e| {
        stack_is_empty(e) && types(e, target).contains(TypeSet::CREATURE)
    });

    let after = types(&engine, target);
    assert!(
        after.contains(TypeSet::ARTIFACT) && after.contains(TypeSet::CREATURE),
        "it becomes an artifact creature, keeping what it already was: {after:?}"
    );
    assert_eq!(
        pt(&engine, target),
        (3, 3),
        "with the body it prints, so no state-based check eats the newcomer"
    );
    assert!(
        !types(&engine, bystander).contains(TypeSet::CREATURE),
        "the Vehicle nobody named is not animated"
    );
    let land = types(&engine, hangar);
    assert!(
        land.contains(TypeSet::LAND) && !land.contains(TypeSet::CREATURE),
        "and the permanent that paid is not the permanent that changed: {land:?}"
    );

    // The offer is the reader: a moment ago the Copter was not a creature and
    // the combat step did not list it, and the animation is all that changed.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&target),
        "an animated 3/3 flier may attack (CR 302.6 is not in its way): {attackers:?}"
    );
    assert!(
        !attackers.contains(&bystander),
        "the Vehicle left as an artifact still may not: {attackers:?}"
    );
}
