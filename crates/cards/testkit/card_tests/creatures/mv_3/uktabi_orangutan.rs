//! `cards/creatures/mv_3/uktabi_orangutan.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Uktabi Orangutan is a {2}{G} 2/2 Ape with "When this creature enters,
/// destroy target artifact." The scenario proves the trigger is asked *after*
/// the creature has resolved rather than as part of casting it, and that the
/// filter is read as "artifact" and neither "permanent" nor "yours": both Sol
/// Rings are offered while the Llanowar Elves beside them are not, and only
/// the one that was named leaves the battlefield. The second Sol Ring and the
/// Elves are the controls — a widened filter would have killed one of them,
/// and nothing about the Ape itself would have looked wrong.
#[test]
fn uktabi_orangutan_destroys_the_artifact_it_was_pointed_at_and_no_other_permanent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                quiet_artifact(),
            ],
        )
        .hand(0, &[uktabi_orangutan()])
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");

    cast_from_hand(&mut engine, p0, uktabi_orangutan());
    // The Ape resolves first; its enters-trigger is what asks for the target.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Ape's controller aims the trigger");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target artifact\" reaches either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "a creature is no artifact, so the Elves are not on the menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the artifact across the table was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, uktabi_orangutan()).is_some(),
        "the Ape itself is on the battlefield, which is where its trigger came from"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the targeted artifact was destroyed and went to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact the trigger did not name is untouched: one target, one destruction"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and nothing that is not an artifact ever moved"
    );
}
