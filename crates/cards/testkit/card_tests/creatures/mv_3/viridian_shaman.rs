//! `cards/creatures/mv_3/viridian_shaman.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Viridian Shaman is a `{2}{G}` 2/2 Elf Shaman whose entire text is "When
/// this creature enters, destroy target artifact." The board carries an
/// artifact under each seat — so "target artifact" is read as the whole table
/// and not as "you control", which is the filter this card is easiest to
/// narrow by accident — plus a creature across it, so the offer proves the
/// type was read rather than skipped. Cast from three Forests and aimed at the
/// opponent's Sol Ring, it shows the destroy resolved: the card is in its
/// *owner's* graveyard, my own artifact is still standing and the Elf beside
/// it never moved.
#[test]
fn viridian_shaman_destroys_an_artifact_across_the_table_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), quiet_artifact()])
        .hand(0, &[viridian_shaman()])
        .battlefield(1, &[quiet_artifact(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_ring = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let their_ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");

    cast_from_hand(&mut engine, p0, viridian_shaman());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert!(
        on_battlefield(&engine, p0, viridian_shaman()).is_some(),
        "the creature is on the battlefield before its enters-trigger is put on the stack"
    );
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the trigger's own target")
    };
    assert_eq!(player, p0, "the Shaman's controller aims its own trigger");
    assert!(
        options.contains(&my_ring) && options.contains(&their_ring),
        "\"target artifact\" reaches either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&their_elf),
        "a creature is not an artifact, so the Elf across the table is no target: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_ring],
            },
        )
        .expect("the artifact the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the targeted artifact left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "and it is in its owner's graveyard — destroyed, not exiled"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact the trigger did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_some(),
        "and neither did the creature beside it"
    );
    assert!(
        on_battlefield(&engine, p0, viridian_shaman()).is_some(),
        "the Shaman survives its own trigger"
    );
}
