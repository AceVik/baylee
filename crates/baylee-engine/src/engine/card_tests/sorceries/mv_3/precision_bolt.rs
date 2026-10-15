// Oracle checklist: Precision Bolt deals 3 damage to any target.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("5f9ea6e4-f5be-4ea2-95b4-fbdc69685ac5")
}

// Any target: player and creature resolve; battle defense damage is currently unsupported
// by the shared engine, so no battle damage outcome is asserted here.
#[test]
fn damages_a_player_for_three() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 17)
        .life(1, 21)
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    cast_from_hand(&mut game, p0, subject());
    let Pending::ChooseTargets { player_options, .. } = game.pending().clone() else {
        panic!("target required")
    };
    assert!(player_options.contains(&p0));
    assert!(player_options.contains(&p1));
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 17);
    assert_eq!(game.state().players[1].life, 18);
}
#[test]
fn damages_a_creature_but_not_a_bystander() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[fangren_hunter(), gray_ogre(), quiet_artifact()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let target = on_battlefield(&game, p1, fangren_hunter()).unwrap();
    let bystander = on_battlefield(&game, p1, gray_ogre()).unwrap();
    let artifact = on_battlefield(&game, p1, quiet_artifact()).unwrap();
    cast_from_hand(&mut game, p0, subject());
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("target required")
    };
    assert!(options.contains(&target));
    assert!(options.contains(&bystander));
    assert!(!options.contains(&artifact));
    aim_at(&mut game, p0, target);
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().object(target).unwrap().damage, 3);
    assert_eq!(game.state().object(bystander).unwrap().damage, 0);
}

#[test]
fn damages_a_planeswalker_by_three_loyalty() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[karn_the_great_creator()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let walker = on_battlefield(&game, p1, karn_the_great_creator()).unwrap();
    assert_eq!(
        game.state()
            .object(walker)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        5
    );
    cast_from_hand(&mut game, p0, subject());
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("any target choice required")
    };
    assert!(options.contains(&walker));
    aim_at(&mut game, p0, walker);
    pass_until(&mut game, stack_is_empty);
    assert_eq!(
        game.state()
            .object(walker)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        2
    );
    assert_eq!(game.state().players[1].life, 20);
}
