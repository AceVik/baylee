//! `cards/creatures/mv_4/ravenous_baboons.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ravenous Baboons` is a four-mana 3/2 creature under `Coverage::Implemented` with an enters-the-battlefield trigger that destroys target nonbasic land.
/// When cast from hand off four Mountains, it enters the battlefield and its trigger asks for a target.
/// Nonbasic lands on the battlefield are valid targets, while basic lands are excluded from the choice.
/// Upon resolution, the targeted nonbasic land is destroyed and put into its owner's graveyard.
#[test]
fn ravenous_baboons_enters_and_destroys_target_nonbasic_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[ravenous_baboons()])
        .battlefield(1, &[badlands(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let their_badlands = on_battlefield(&engine, p1, badlands()).expect("Badlands deployed");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("Forest deployed");

    cast_from_hand(&mut engine, p0, ravenous_baboons());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "caster names the target");
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(
        options.contains(&their_badlands),
        "nonbasic Badlands is an offered target: {options:?}"
    );
    assert!(
        !options.contains(&their_forest),
        "basic Forest is not a nonbasic land: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_badlands],
            },
        )
        .expect("targeting Badlands is legal");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, ravenous_baboons()).is_some(),
        "Ravenous Baboons resolved onto the battlefield"
    );
    assert!(
        on_battlefield(&engine, p1, badlands()).is_none(),
        "targeted Badlands left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, badlands()).is_some(),
        "destroyed Badlands was placed in owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "basic Forest was untouched"
    );
}
