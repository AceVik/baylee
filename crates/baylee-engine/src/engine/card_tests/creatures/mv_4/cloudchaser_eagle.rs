//! `cards/creatures/mv_4/cloudchaser_eagle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cloudchaser Eagle prints "Flying" and "When this creature enters, destroy
/// target enchantment" for `{3}{W}`. Both printed halves need a board that can
/// tell them from their neighbours, so the table carries **two** of the
/// opponent's enchantments — one target is not a sweeper — beside an artifact
/// that "target enchantment" must decline, and the four Plains are the exact
/// `{3}{W}` so the creature is cast out of a pool the lands actually filled.
/// The trigger is read where CR 603.3b puts it: the Eagle is already on the
/// battlefield while the question is still open, and only afterwards is one
/// enchantment gone from the battlefield and in its owner's graveyard while
/// the other never moved.
#[test]
fn cloudchaser_eagle_flies_in_and_destroys_exactly_one_enchantment() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4411, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[cloudchaser_eagle()])
        // Two enchantments, so "target enchantment" is one and not all of
        // them, and an artifact that is not an enchantment at all.
        .battlefield(
            1,
            &[their_enchantment(), their_enchantment(), quiet_artifact()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Plains tap for the {{3}}{{W}} the Eagle costs"
    );
    cast_with_floating(&mut engine, p0, cloudchaser_eagle());

    // The spell resolves, the creature enters, and the enters-trigger asks
    // what to destroy.
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
    assert_eq!(player, p0, "the Eagle's controller aims the trigger");
    assert_eq!(
        (min, max),
        (1, 1),
        "exactly one enchantment, no more and no fewer"
    );
    let doom = on_battlefield(&engine, p1, their_enchantment()).expect("an enchantment is out");
    let spare = all_on_battlefield(&engine, p1, their_enchantment())
        .into_iter()
        .find(|id| *id != doom)
        .expect("the second copy stands beside it");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("the artifact is out");
    assert!(
        options.contains(&doom) && options.contains(&spare),
        "\"target enchantment\" reaches both across the table: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "an artifact is no enchantment: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, cloudchaser_eagle()).is_some(),
        "the trigger is asked after the creature has entered, not on the way"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doom],
            },
        )
        .expect("the enchantment the question offered is the one that dies");
    pass_until(&mut engine, stack_is_empty);

    let eagle = on_battlefield(&engine, p0, cloudchaser_eagle()).expect("the Eagle resolved");
    assert_eq!(pt(&engine, eagle), (2, 2), "the printed 2/2 body");
    assert!(
        keywords(&engine, eagle).contains(KeywordSet::FLYING),
        "and the printed flying line reaches the permanent"
    );
    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_some(),
        "one enchantment was named, so the other is still standing"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "and the one that was named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the artifact the trigger declined never moved"
    );
}
