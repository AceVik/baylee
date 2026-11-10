// Oracle checklist: own grave creature only; opponent grave creature, own grave artifact and battlefield excluded; exact card to own hand, bystanders stay; sorcery cast.
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("cf6676b7-b40a-46bd-b166-bd3cbb49d80d")
}

#[test]
fn returns_only_own_graveyard_creature_to_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[swamp(), fangren_hunter(), wild_elephant(), quiet_artifact()],
        )
        .battlefield(1, &[fangren_hunter()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let target = on_battlefield(&game, p0, fangren_hunter()).unwrap();
    let artifact = on_battlefield(&game, p0, quiet_artifact()).unwrap();
    let theirs = on_battlefield(&game, p1, fangren_hunter()).unwrap();
    let live = on_battlefield(&game, p0, wild_elephant()).unwrap();
    bury(&mut game, &[target, artifact, theirs]);
    game.refresh_offer();
    cast_from_hand(&mut game, p0, subject());
    let options = aim_at(&mut game, p0, target);
    assert_eq!(options, vec![target]);
    assert!(!options.contains(&artifact));
    assert!(!options.contains(&theirs));
    assert!(!options.contains(&live));
    pass_until(&mut game, stack_is_empty);
    assert_eq!(in_hand(&game, p0, fangren_hunter()), Some(target));
    assert!(in_graveyard(&game, p0, fangren_hunter()).is_none());
    assert!(in_graveyard(&game, p1, fangren_hunter()).is_some());
    assert!(in_graveyard(&game, p0, quiet_artifact()).is_some());
    assert_eq!(on_battlefield(&game, p0, wild_elephant()), Some(live));
}
