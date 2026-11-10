//! `cards/lands/utility/unholy_grotto.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Unholy Grotto: "{T}: Add {C}." / "{B}, {T}: Put target Zombie card from your graveyard on top of your library."
/// With a Zombie card in the graveyard and {B} mana available from a Swamp, ability 1 is activated.
/// The targeted Zombie card is returned from the graveyard and placed on top of the library.
#[test]
fn unholy_grotto_puts_zombie_from_graveyard_on_top_of_library() {
    let p0 = PlayerId::new(0);
    let festering_goblin = card_index("66fb4764-d309-4c30-a2a4-474f9030dc87");
    let mut engine = Duel::new(109, festering_goblin)
        .battlefield(0, &[unholy_grotto(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let grotto = on_battlefield(&engine, p0, unholy_grotto()).expect("Grotto deployed");
    let gy = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    assert_eq!(gy.len(), 1);
    let zombie = gy[0];

    tap_mana_except(&mut engine, p0, grotto);
    activate(&mut engine, p0, unholy_grotto(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&zombie));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![zombie],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        0,
        "Zombie left the graveyard"
    );
    let lib = engine.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(
        lib.last().copied(),
        Some(zombie),
        "Zombie is on top of library"
    );
    assert!(is_tapped(&engine, grotto));
}
