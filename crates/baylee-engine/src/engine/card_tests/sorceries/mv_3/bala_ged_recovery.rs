//! `cards/sorceries/mv_3/bala_ged_recovery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Bala Ged Recovery` // `Bala Ged Sanctuary` (`Coverage::Implemented`): "Return target
/// card from your graveyard to your hand. // This land enters tapped. {T}: Add {G}."
///
/// Marked `Coverage::Implemented`, casting the front face for `{2}{G}` targets a card in
/// your graveyard via `TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::You)`.
/// The test verifies that only cards in the caster's graveyard are offered, choosing the
/// seeded card and confirming it returns to hand upon resolution.
#[test]
fn bala_ged_recovery_returns_target_card_from_own_graveyard_to_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(473, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[bala_ged_recovery()])
        .start();
    keep_mulligans(&mut engine);

    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);

    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let p0_target = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    let p1_other = engine.state().zones.list(ZoneLocation::Graveyard(p1))[0];

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, bala_ged_recovery());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&p0_target),
        "card in caster's graveyard is a legal target"
    );
    assert!(
        !options.contains(&p1_other),
        "card in opponent's graveyard is not a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_target],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&p0_target),
        "targeted card returned to caster's hand"
    );
    assert!(
        in_graveyard(&engine, p0, bala_ged_recovery()).is_some(),
        "Bala Ged Recovery resolved to graveyard"
    );
}
