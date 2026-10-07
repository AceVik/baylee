//! `cards/instants/mv_1/oxidize.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Oxidize — {G} instant: "Destroy target artifact. It can't be regenerated."
///
/// The filter is the whole card, and it needs a witness on each side: an
/// artifact under the caster's own control and one across the table are both
/// on the offer, because the printing names no controller — while the Forest
/// beside them is a permanent and no artifact, which is what says
/// `Filter::ARTIFACT` was read rather than skipped. Only the artifact the
/// question was answered with leaves, and it goes to its *owner's* graveyard,
/// so a resolve that had swept the type would fail the survivor below. The
/// second printed sentence has nothing to act on: no card in the pool
/// regenerates anything.
#[test]
fn oxidize_destroys_the_artifact_it_names_and_leaves_its_neighbour_standing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), quiet_artifact()])
        .battlefield(1, &[quiet_artifact(), forest()])
        .hand(0, &[oxidize()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my artifact is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their artifact is out");
    let land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    assert!(
        types(&engine, mine).contains(TypeSet::ARTIFACT),
        "the permanent the filter has to read is the artifact it prints"
    );

    // `{G}` comes off the two Forests; the offer at the target question is
    // what the card is, so nothing is asserted before the mana is floating.
    cast_from_hand(&mut engine, p0, oxidize());
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
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster aims it");
    assert_eq!((min, max), (1, 1), "\"target artifact\" is one artifact");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target artifact\" names no controller, so both sides of the table are \
         on the menu: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a land is a permanent and no artifact: {options:?}"
    );
    assert_eq!(options.len(), 2, "those two artifacts are the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the artifact the question offered is the one that is named");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the destroyed artifact is in its *owner's* graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and no longer on the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "nor did anything that is not an artifact"
    );
    assert!(
        in_graveyard(&engine, p0, oxidize()).is_some(),
        "the instant itself resolved and is in its caster's graveyard"
    );
}
