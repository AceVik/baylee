//! `cards/lands/utility/hall_of_heliod_s_generosity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hall of Heliod's Generosity: "{T}: Add {C}." / "{1}{W}, {T}: Put target enchantment card from your graveyard on top of your library."
/// With an enchantment card in the graveyard and mana from two Plains, ability 1 targets the enchantment.
/// Upon resolution, the enchantment card is moved from the graveyard to the top of the library.
#[test]
fn hall_of_heliod_s_generosity_puts_enchantment_on_top_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(123, luminarch_ascension())
        .battlefield(0, &[hall_of_heliod_s_generosity(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let hall = on_battlefield(&engine, p0, hall_of_heliod_s_generosity()).expect("Hall deployed");
    let gy = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    assert_eq!(gy.len(), 1);
    let target_ench = gy[0];

    tap_mana_except(&mut engine, p0, hall);
    activate(&mut engine, p0, hall_of_heliod_s_generosity(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&target_ench));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_ench],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        0,
        "enchantment left the graveyard"
    );
    let lib = engine.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(
        lib.last().copied(),
        Some(target_ench),
        "enchantment on top of library"
    );
    assert!(is_tapped(&engine, hall));
}
