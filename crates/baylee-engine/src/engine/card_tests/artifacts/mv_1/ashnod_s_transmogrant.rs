//! `cards/artifacts/mv_1/ashnod_s_transmogrant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ashnod's Transmogrant: "{T}, Sacrifice this artifact: Put a +1/+1 counter
/// on target nonartifact creature. That creature becomes an artifact in
/// addition to its other types."
///
/// The 2007 ruling is the second sentence: it is the ability and not the
/// counter that makes the creature an artifact, so removing the counter
/// leaves the type behind. The target menu is the first sentence's filter:
/// a nonartifact creature on either side, an artifact creature on none.
#[test]
fn ashnod_s_transmogrant_adds_a_counter_and_a_lasting_artifact_type() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[ashnod_s_transmogrant(), llanowar_elves(), ornithopter()],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let thopter = on_battlefield(&engine, p0, ornithopter()).expect("the Thopter is out");
    let can =
        on_battlefield(&engine, p0, ashnod_s_transmogrant()).expect("the Transmogrant is out");

    activate(&mut engine, p0, ashnod_s_transmogrant(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the ability asks for its target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!((player, min, max), (p0, 1, 1));
    assert!(
        options.contains(&elf) && options.contains(&theirs),
        "a nonartifact creature on either side: {options:?}"
    );
    assert!(
        !options.contains(&thopter),
        "an artifact creature is not a nonartifact creature: {options:?}"
    );
    assert!(
        !options.contains(&can),
        "the Transmogrant is an artifact and not a creature: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(counters_on(&engine, elf, CounterKind::P1P1), 1);
    assert!(
        types(&engine, elf).intersects(TypeSet::ARTIFACT),
        "the target becomes an artifact in addition to its other types"
    );
    assert!(
        in_graveyard(&engine, p0, ashnod_s_transmogrant()).is_some(),
        "sacrificing it paid the cost"
    );
    assert!(
        !types(&engine, theirs).intersects(TypeSet::ARTIFACT),
        "the other Elf was not the target"
    );

    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::replacement::remove_counters(state, elf, CounterKind::P1P1, 1);
    state.refresh_characteristics();
    engine.refresh_offer();
    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        0,
        "the counter is gone"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "and so is its +1/+1");
    assert!(
        types(&engine, elf).intersects(TypeSet::ARTIFACT),
        "but the artifact type the ability added stays (2007 ruling)"
    );
}
