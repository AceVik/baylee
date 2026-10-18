//! `cards/lands/utility/northampton_farm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Northampton Farm prints `{{T}}: Add {{C}}.`, `{{1}}, {{T}}: Exile target creature you own.`, and `{{2}}, {{T}}, Sacrifice this land: Return a creature card exiled with this land to the battlefield under your control. Return each other card exiled with this land to its owner's hand.`
///
/// With Northampton Farm, two basic lands, and a creature on the battlefield under `PlayerId::new(0)`, and an opponent's creature across the table, floating `{{2}}` while keeping Northampton Farm untapped shows that ability 0 and ability 1 are offered while ability 2 is offered when three or more cards are exiled with Northampton Farm.
/// Activating ability 1 costs `{{1}}` and `{{T}}`, targets only the owned creature, and exiles it; activating ability 2 returns one exiled card to the battlefield tapped and the rest to their owner's hand.
#[test]
#[allow(clippy::too_many_lines)]
fn northampton_farm_taps_for_colorless_and_exiles_owned_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                northampton_farm(),
                forest(),
                forest(),
                young_wolf(),
                ondu_cleric(),
            ],
        )
        .battlefield(1, &[young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let farm = on_battlefield(&engine, p0, northampton_farm()).expect("farm on battlefield");
    let my_wolf = on_battlefield(&engine, p0, young_wolf()).expect("my wolf on battlefield");
    let bears = on_battlefield(&engine, p0, ondu_cleric()).expect("my cleric on battlefield");
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
        legal.abilities.contains(&(farm, 2)),
        "ability 2 ({{2}}, {{T}}, Sacrifice this land) is offered with {{2}} floating"
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

    // Exile the second owned creature with Northampton Farm, then activate ability 2.
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(farm)
        .unwrap()
        .status
        .remove(crate::object::Status::TAPPED);
    engine.refresh_offer();
    activate(&mut engine, p0, northampton_farm(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Exile);

    // Untap farm, add {2}, and activate ability 2: choose `bears` for battlefield, `my_wolf` goes to hand.
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(farm)
        .unwrap()
        .status
        .remove(crate::object::Status::TAPPED);
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    engine.refresh_offer();
    activate(&mut engine, p0, northampton_farm(), 2);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(bears).unwrap().zone,
        Zone::Battlefield,
        "chosen creature card exiled with Northampton Farm returns to the battlefield"
    );
    assert_eq!(
        engine.state().object(my_wolf).unwrap().zone,
        Zone::Hand,
        "each other card exiled with Northampton Farm returns to its owner's hand"
    );
}
