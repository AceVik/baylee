//! `cards/instants/mv_3/verdigris.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Verdigris is a {2}{G} instant under `Coverage::Implemented` that destroys target artifact.
/// When cast, artifacts are offered as legal targets while non-artifact creatures are excluded.
/// Upon resolution, the targeted artifact is destroyed and placed into its owner's graveyard.
/// Other permanents on the battlefield remain unaffected.
#[test]
fn verdigris_destroys_target_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[verdigris()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let card = in_hand(&engine, p0, verdigris()).expect("Verdigris is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool cannot pay {{2}}{{G}}"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests produce three mana"
    );
    cast_with_floating(&mut engine, p0, verdigris());

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
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");

    assert_eq!(player, p0, "the casting player chooses the target");
    assert_eq!((min, max), (1, 1), "one target artifact is required");
    assert!(
        options.contains(&rock),
        "artifact is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "creature is not an artifact: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("target artifact selected");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "targeted artifact is in graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "targeted artifact is no longer on battlefield"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "bystander creature remains on battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, verdigris()).is_some(),
        "Verdigris is in graveyard"
    );
}
