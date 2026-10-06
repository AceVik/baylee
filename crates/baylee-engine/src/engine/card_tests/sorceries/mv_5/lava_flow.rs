//! `cards/sorceries/mv_5/lava_flow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lava Flow is a {3}{R}{R} sorcery under `Coverage::Implemented` that destroys target creature or land.
/// When cast, both creatures and lands are presented as legal targets while artifacts are excluded.
/// Resolving the spell destroys the chosen target and puts it into its owner's graveyard.
/// Bystanders not chosen by the spell remain safely on the battlefield.
#[test]
fn lava_flow_destroys_target_creature_or_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[lava_flow()])
        .battlefield(1, &[forest(), llanowar_elves(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, lava_flow()).expect("Lava Flow is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "empty mana pool cannot pay {{3}}{{R}}{{R}}"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains produce five mana"
    );
    cast_with_floating(&mut engine, p0, lava_flow());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    let target_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let target_creature = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");

    assert_eq!(player, p0, "the caster chooses the target");
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(
        options.contains(&target_land),
        "target land is an offered target: {options:?}"
    );
    assert!(
        options.contains(&target_creature),
        "target creature is an offered target: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "artifact is neither creature nor land: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .expect("destroying target land is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "targeted land is in the graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "targeted land left the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&target_creature),
        "unselected creature remains on the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&rock),
        "unselected artifact remains on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, lava_flow()).is_some(),
        "Lava Flow went to graveyard after resolution"
    );
}
