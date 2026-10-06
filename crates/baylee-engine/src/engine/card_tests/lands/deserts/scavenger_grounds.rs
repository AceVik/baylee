//! `cards/lands/deserts/scavenger_grounds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scavenger Grounds: "{2}, {T}, Sacrifice a Desert: Exile all graveyards."
/// Both players have cards in their graveyards, and two Forests pay the generic cost.
/// Scavenger Grounds sacrifices itself as the chosen Desert, exiling all graveyards.
#[test]
fn scavenger_grounds_sacrifices_a_desert_to_exile_all_graveyards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(63, forest())
        .battlefield(0, &[scavenger_grounds(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 2);
    seed_graveyard(&mut engine, p1, 2);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        2
    );

    let sg = on_battlefield(&engine, p0, scavenger_grounds()).expect("Scavenger Grounds deployed");
    tap_mana_except(&mut engine, p0, sg);

    activate(&mut engine, p0, scavenger_grounds(), 1);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice cost choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert!(
        options.contains(&sg),
        "Scavenger Grounds is a Desert and can be sacrificed"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![sg] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty(),
        "p0 graveyard is completely exiled"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .is_empty(),
        "p1 graveyard is completely exiled"
    );
    assert!(on_battlefield(&engine, p0, scavenger_grounds()).is_none());
}
