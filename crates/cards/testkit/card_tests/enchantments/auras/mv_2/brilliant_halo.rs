#[allow(clippy::wildcard_imports)]
use super::*;

fn brilliant_halo() -> CardIndex {
    card_index("8f68f6d5-1953-4ccd-8af0-7d0a69139556")
}

#[test]
fn brilliant_halo_buffs_enchanted_creature_and_returns_to_hand_on_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[brilliant_halo()])
        .battlefield(0, &[plains(), plains(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, brilliant_halo());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 4));
    let halo = on_battlefield(&engine, p0, brilliant_halo()).expect("seated");

    kill(&mut engine, halo);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, brilliant_halo()).is_some());
}

#[test]
fn brilliant_halo_does_not_return_when_discarded_from_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[brilliant_halo()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hand_to_graveyard(&mut engine, p0, brilliant_halo());
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, brilliant_halo()).is_some());
    assert!(in_hand(&engine, p0, brilliant_halo()).is_none());
}
