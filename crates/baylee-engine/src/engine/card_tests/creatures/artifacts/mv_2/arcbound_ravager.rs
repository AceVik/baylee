//! `cards/creatures/artifacts/mv_2/arcbound_ravager.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Modular supplies the entry counter; sacrificing an artifact grows Ravager.
#[allow(clippy::too_many_lines)] // one activation, the menu it offers and the counter it leaves
#[test]
fn arcbound_ravager_eats_the_artifact_you_name_and_grows_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arcbound_ravager(), quiet_artifact(), forest()])
        .battlefield(1, &[quiet_artifact()])
        .start();

    let ravager =
        on_battlefield(&engine, p0, arcbound_ravager()).expect("the Ravager is on the table");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring stands");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring stands");
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert_eq!(
        counters_on(&engine, ravager, CounterKind::P1P1),
        1,
        "the counter supplied by modular"
    );
    assert_eq!(pt(&engine, ravager), (1, 1), "a printed 0/0 with one on it");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the seat with the Ravager");
    let offered = deeds(&legal, &[ravager]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(0))]),
        "there is an artifact to eat, so the one line the Ravager prints is \
         offered: {offered:?}"
    );

    activate(&mut engine, p0, arcbound_ravager(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the sacrifice is chosen before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, and the cost asks once");
    assert!(
        options.contains(&ravager),
        "the Ravager is an artifact, so it is on its own menu: {options:?}"
    );
    assert!(
        options.contains(&rock),
        "and so is the Sol Ring beside it: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Forest is a land and no artifact: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "a seat sacrifices only what it controls: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("the artifact the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "the sacrificed artifact left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "and is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the Sol Ring across the table never moved"
    );
    assert!(
        on_battlefield(&engine, p0, arcbound_ravager()).is_some(),
        "the Ravager ate something else and still stands"
    );
    assert_eq!(
        counters_on(&engine, ravager, CounterKind::P1P1),
        2,
        "the harness' counter plus the one the ability put there"
    );
    assert_eq!(
        pt(&engine, ravager),
        (2, 2),
        "a printed 0/0 with two +1/+1 counters"
    );
}
