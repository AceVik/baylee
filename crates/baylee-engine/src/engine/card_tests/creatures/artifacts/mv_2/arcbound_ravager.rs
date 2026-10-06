//! `cards/creatures/artifacts/mv_2/arcbound_ravager.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Arcbound Ravager — {2} — is printed `0/0`, and its second line, modular,
/// is the `Coverage::Partial` gap: a `0/0` with nothing on it is a corpse
/// (CR 704.5f) before any seat is offered priority, so the harness plants the
/// counter modular would have brought and the card is played from there.
///
/// The one activation left is the whole test, and every half of it is the
/// engine's answer rather than the card's: the cost names no artifact, so the
/// engine asks which one, and the menu is what reads the filter — this seat's
/// two artifacts, the Ravager itself and the Sol Ring beside it, and neither
/// the Forest under the same seat nor the Sol Ring across the table, because
/// "an artifact" paid to a cost still means one you control (CR 701.21a).
/// The eaten Sol Ring ends in its owner's graveyard and the +1/+1 counter
/// lands on the Ravager, which is the only place this printing ever puts one.
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
    // Modular's replacement is the missing half, so the counter it would have
    // entered with comes from the harness — before the first state-based
    // check, which is the only moment a `0/0` is still on the battlefield.
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        crate::replacement::put_counters(state, ravager, CounterKind::P1P1, 1);
    }

    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert_eq!(
        counters_on(&engine, ravager, CounterKind::P1P1),
        1,
        "the counter modular cannot supply, planted by the harness"
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
