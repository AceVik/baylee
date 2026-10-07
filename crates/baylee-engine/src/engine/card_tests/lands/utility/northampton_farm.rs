//! `cards/lands/utility/northampton_farm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Northampton Farm prints `{{T}}: Add {{C}}.`, `{{1}}, {{T}}: Exile target creature you own.`, and `{{2}}, {{T}}, Sacrifice this land: Return a creature card exiled with this land to the battlefield under your control. Return each other card exiled with this land to its owner's hand.`
///
/// Under `Coverage::Partial`, the return clause is omitted because no effect variant partitions exiled cards between battlefield and hand.
/// With Northampton Farm, two basic lands, and a creature on the battlefield under `PlayerId::new(0)`, and an opponent's creature across the table, floating `{{2}}` while keeping Northampton Farm untapped shows that ability 0 and ability 1 are offered while ability 2 is omitted from `legal.abilities`.
/// Activating ability 1 costs `{{1}}` and `{{T}}`, targets only the owned creature, and exiles it.
#[test]
fn northampton_farm_taps_for_colorless_and_exiles_owned_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[northampton_farm(), forest(), forest(), young_wolf()])
        .battlefield(1, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let farm = on_battlefield(&engine, p0, northampton_farm()).expect("farm on battlefield");
    let my_wolf = on_battlefield(&engine, p0, young_wolf()).expect("my wolf on battlefield");
    let their_wolf = on_battlefield(&engine, p1, young_wolf()).expect("their wolf on battlefield");

    // Float {{2}} while keeping Northampton Farm untapped.
    tap_mana_except(&mut engine, p0, farm);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(!is_tapped(&engine, farm));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(farm, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(farm, 1)),
        "ability 1 ({{1}}, {{T}}: Exile target creature you own) is offered"
    );
    assert!(
        !legal.abilities.contains(&(farm, 2)),
        "ability 2 is omitted under `Coverage::Partial` even with {{2}} floating"
    );

    activate(&mut engine, p0, northampton_farm(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&my_wolf),
        "target creature you own offers your own creature"
    );
    assert!(
        !options.contains(&their_wolf),
        "target creature you own rejects opponent's creature"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_wolf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, young_wolf()).is_none());
    assert_eq!(
        engine.state().object(my_wolf).expect("wolf exists").zone,
        Zone::Exile
    );
    assert!(is_tapped(&engine, farm));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
}
