// Oracle checklist: Destroy target nonblack creature.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("73b6694a-47d4-417a-af9b-470da14b5b0e")
}

// Nonblack creatures on either side are legal; black creatures and noncreatures are excluded.
#[test]
fn destroys_only_a_target_nonblack_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), gray_ogre()])
        .battlefield(
            1,
            &[
                wild_elephant(),
                gray_ogre(),
                lotleth_troll(),
                quiet_artifact(),
            ],
        )
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let own = on_battlefield(&game, p0, gray_ogre()).unwrap();
    let target = on_battlefield(&game, p1, wild_elephant()).unwrap();
    let bystander = on_battlefield(&game, p1, gray_ogre()).unwrap();
    let artifact = on_battlefield(&game, p1, quiet_artifact()).unwrap();
    let black = on_battlefield(&game, p1, lotleth_troll()).unwrap();
    cast_from_hand(&mut game, p0, subject());
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("target required")
    };
    assert!(options.contains(&own));
    assert!(options.contains(&target));
    assert!(!options.contains(&artifact));
    assert!(!options.contains(&black), "black creature excluded");
    aim_at(&mut game, p0, target);
    pass_until(&mut game, stack_is_empty);
    assert!(in_graveyard(&game, p1, wild_elephant()).is_some());
    assert!(game.state().object(bystander).is_some());
}
