//! `cards/creatures/mv_5/barbarian_riftcutter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Barbarian Riftcutter` is a 3/3 Human Barbarian costing `{4}{R}` under `Coverage::Implemented`.
/// It prints "{R}, Sacrifice this creature: Destroy target land."
/// When activated off floating red mana, targets are chosen first while the creature remains
/// on the battlefield. Once the target land is chosen, paying the cost sacrifices the creature.
/// Upon resolution, the targeted land is destroyed and sent to its owner's graveyard.
#[test]
fn barbarian_riftcutter_sacrifices_itself_to_destroy_target_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[barbarian_riftcutter(), mountain()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let riftcutter =
        on_battlefield(&engine, p0, barbarian_riftcutter()).expect("Riftcutter is present");
    let their_land = on_battlefield(&engine, p1, mountain()).expect("opponent controls a land");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "Mountain produced one red mana for the activation cost"
    );

    activate(&mut engine, p0, barbarian_riftcutter(), 0);

    // CR 601.2c: Target is chosen before costs are paid; Riftcutter is still on the battlefield.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&their_land),
        "opponent's land is a legal target: {options:?}"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&riftcutter),
        "Riftcutter has not yet been sacrificed while choosing targets"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_land],
                players: Vec::new(),
            },
        )
        .expect("targeting opponent's land is legal");

    // CR 601.2h: Cost is now paid, so Riftcutter is in the graveyard.
    assert!(
        on_battlefield(&engine, p0, barbarian_riftcutter()).is_none(),
        "Riftcutter was sacrificed as cost"
    );
    assert!(
        in_graveyard(&engine, p0, barbarian_riftcutter()).is_some(),
        "sacrificed Riftcutter is in caster's graveyard"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, mountain()).is_none(),
        "targeted land was destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, mountain()).is_some(),
        "destroyed land is in opponent's graveyard"
    );
}
