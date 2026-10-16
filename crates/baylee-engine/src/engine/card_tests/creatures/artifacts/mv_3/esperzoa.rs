//! `cards/creatures/artifacts/mv_3/esperzoa.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn esperzoa() -> CardIndex {
    card_index("c9a188f9-2e65-4a66-826f-0150f25730af")
}

/// Esperzoa — flying 4/3; "At the beginning of your upkeep, return an
/// artifact you control to its owner's hand." The choice includes the
/// Esperzoa itself; sending the other artifact home leaves the Esperzoa.
#[test]
fn esperzoa_returns_an_artifact_you_control_each_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[esperzoa(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    let zoa = on_battlefield(&engine, p0, esperzoa()).expect("seated");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("seated");
    assert_eq!(pt(&engine, zoa), (4, 3));
    assert!(keywords(&engine, zoa).contains(KeywordSet::FLYING));

    // Seat 0's first upkeep is the first thing that happens.
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::ChooseCards { .. }));
    assert_eq!(engine.state().turn.step, Step::Upkeep);
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0);
    assert!(options.contains(&zoa) && options.contains(&ring));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![ring] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, quiet_artifact()).is_some(), "the other artifact went home");
    assert!(on_battlefield(&engine, p0, quiet_artifact()).is_none());
    assert!(on_battlefield(&engine, p0, esperzoa()).is_some());
}
