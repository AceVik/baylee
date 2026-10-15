// Oracle checklist: When this creature enters, it deals 1 damage to target creature and 1 damage to you.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("be7a38a3-c27f-4917-a59c-763d0b36e427")
}

// Entry requires creature target, deals one to it and separately one to its controller.
#[test]
fn entry_damages_creature_and_self_exactly_once() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 15)
        .life(1, 18)
        .battlefield(0, &[mountain()])
        .battlefield(1, &[gray_ogre(), quiet_artifact()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let target = on_battlefield(&game, p1, gray_ogre()).unwrap();
    let artifact = on_battlefield(&game, p1, quiet_artifact()).unwrap();
    cast_from_hand(&mut game, p0, subject());
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = game.pending().clone()
    else {
        unreachable!()
    };
    assert!(options.contains(&target));
    assert!(!options.contains(&artifact));
    assert!(player_options.is_empty());
    aim_at(&mut game, p0, target);
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().object(target).unwrap().damage, 1);
    assert_eq!(game.state().players[0].life, 14);
    assert_eq!(game.state().players[1].life, 18);
}
