// Oracle checklist: When this creature enters, put a +1/+1 counter on target creature and you gain 1 life.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("7a70faf5-cc04-402b-92e3-67746788f510")
}

// Entry targets any creature, including opponent's; counter is permanent and life goes to controller.
#[test]
fn entry_counters_target_creature_and_gains_one_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 13)
        .life(1, 17)
        .battlefield(0, &[forest(), gray_ogre()])
        .battlefield(1, &[wild_elephant(), quiet_artifact()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let own = on_battlefield(&game, p0, gray_ogre()).unwrap();
    let theirs = on_battlefield(&game, p1, wild_elephant()).unwrap();
    let artifact = on_battlefield(&game, p1, quiet_artifact()).unwrap();
    cast_from_hand(&mut game, p0, subject());
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&own));
    assert!(options.contains(&theirs));
    assert!(!options.contains(&artifact));
    aim_at(&mut game, p0, theirs);
    pass_until(&mut game, stack_is_empty);
    assert_eq!(
        game.state()
            .object(theirs)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(pt(&game, theirs), (4, 4));
    assert_eq!(pt(&game, own), (2, 2));
    assert_eq!(game.state().players[0].life, 14);
    assert_eq!(game.state().players[1].life, 17);
}
