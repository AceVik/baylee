//! `cards/lands/utility/petrified_field.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Petrified Field: "{T}: Add {C}." / "{T}, Sacrifice this land: Return target land card from your graveyard to your hand."
/// With a Forest card in the graveyard, Petrified Field's sacrifice ability targets the Forest.
/// Upon resolution, Petrified Field is in the graveyard and the Forest card is returned to hand.
#[test]
fn petrified_field_returns_land_from_graveyard_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(121, forest())
        .battlefield(0, &[petrified_field()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let _field = on_battlefield(&engine, p0, petrified_field()).expect("Field deployed");
    let gy = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    assert_eq!(gy.len(), 1);
    let target_land = gy[0];

    activate(&mut engine, p0, petrified_field(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&target_land));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, petrified_field()).is_none());
    assert!(in_graveyard(&engine, p0, petrified_field()).is_some());
    assert!(in_hand(&engine, p0, forest()).is_some());
}
