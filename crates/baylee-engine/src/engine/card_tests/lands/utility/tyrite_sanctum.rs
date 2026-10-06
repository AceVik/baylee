//! `cards/lands/utility/tyrite_sanctum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Tyrite Sanctum` prints `{{T}}: Add {{C}}.`, `{{2}}, {{T}}: Target legendary creature becomes a God in addition to its other types. Put a +1/+1 counter on it.`, and `{{4}}, {{T}}, Sacrifice this land: Put an indestructible counter on target God.`
///
/// Under `Coverage::Partial`, the first two abilities are built while the indestructible counter ability is omitted.
/// Floating `{{4}}` mana confirms that ability 1 is offered while ability 2 is withheld under `Coverage::Partial`.
/// Activating ability 1 targeting `thorin_oakenshield()` makes it a God, places a `+1/+1` counter on it, and leaves `Tyrite Sanctum` tapped.
#[test]
fn tyrite_sanctum_makes_legendary_creature_a_god_and_adds_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                tyrite_sanctum(),
                thorin_oakenshield(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sanctum = on_battlefield(&engine, p0, tyrite_sanctum()).expect("sanctum on battlefield");
    let thorin = on_battlefield(&engine, p0, thorin_oakenshield()).expect("thorin on battlefield");
    assert_eq!(pt(&engine, thorin), (3, 2));

    // Float {{4}} green mana from basic forests while keeping Tyrite Sanctum untapped.
    tap_mana_except(&mut engine, p0, sanctum);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    assert!(!is_tapped(&engine, sanctum));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(sanctum, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(sanctum, 1)),
        "ability 1 is offered"
    );
    assert!(
        !legal.abilities.contains(&(sanctum, 2)),
        "ability 2 is omitted under `Coverage::Partial` despite floating {{4}}"
    );

    activate(&mut engine, p0, tyrite_sanctum(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&thorin), "Thorin is a legendary creature");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![thorin],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, thorin), (4, 3));
    assert_eq!(counters_on(&engine, thorin, CounterKind::P1P1), 1);
    assert!(
        engine
            .state()
            .object(thorin)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::GOD),
        "Thorin gained the God subtype"
    );
    assert!(is_tapped(&engine, sanctum));
}
