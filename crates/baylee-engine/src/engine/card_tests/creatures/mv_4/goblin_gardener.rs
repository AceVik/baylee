//! `cards/creatures/mv_4/goblin_gardener.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Gardener is a `{3}{R}` 2/1 printing one sentence: "When this
/// creature dies, destroy target land." The scenario plays both halves, so the
/// Gardener is cast for its printed cost and then really dies, and the trigger
/// it leaves behind asks its controller for a land. The menu is where the
/// filter is read: a Mountain of mine and the Island across the table are both
/// on it — the card says "target land" and not "a land you don't control" —
/// while the Sol Ring beside them is a permanent and no land. The land that is
/// named is the only permanent that moves, and it goes to its *owner's*
/// graveyard.
#[test]
fn goblin_gardener_dying_destroys_the_land_its_controller_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                quiet_artifact(),
            ],
        )
        .hand(0, &[goblin_gardener()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {3}{R} off the four Mountains, so what dies below is a permanent that
    // was really cast rather than one the harness placed.
    cast_from_hand(&mut engine, p0, goblin_gardener());
    pass_until(&mut engine, stack_is_empty);
    let gardener = on_battlefield(&engine, p0, goblin_gardener()).expect("the Gardener resolved");
    assert_eq!(pt(&engine, gardener), (2, 1), "the body the card prints");
    let mine = on_battlefield(&engine, p0, mountain()).expect("my Mountain is out");
    let theirs = on_battlefield(&engine, p1, island()).expect("their Island is out");
    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Sol Ring is out");

    // The harness kills it the way a state-based action does, because what is
    // under test is the dying and not the removal: a removal spell would put
    // its own colours, its own cost and its own target legality between the
    // board and the sentence. The death is a trigger, so it is collected, put
    // on the stack and asks its controller for a land before anybody receives
    // priority again (CR 603.3d).
    bury(&mut engine, &[gardener]);
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
        unreachable!("pass_until stopped on nothing else")
    };
    assert_eq!(player, p0, "the dead Gardener's controller names the land");
    assert_eq!((min, max), (1, 1), "one land, and the trigger asks once");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target land\" is any land, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&ring),
        "the Sol Ring is a permanent and no land: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the land across the table was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, island()).is_some(),
        "the land the trigger named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, island()).is_none(),
        "and it left the battlefield, which is what destroy means"
    );
    assert!(
        in_graveyard(&engine, p0, goblin_gardener()).is_some(),
        "the Gardener that died is where the trigger came from"
    );
    assert!(
        in_graveyard(&engine, p0, mountain()).is_none(),
        "the land the ability did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "and neither did the permanent that is no land"
    );
}
