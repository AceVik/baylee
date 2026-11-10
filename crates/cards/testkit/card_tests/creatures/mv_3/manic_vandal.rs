//! Oracle: “When this creature enters, destroy target artifact.”
#[allow(clippy::wildcard_imports)]
use super::*;

fn manic_vandal() -> CardIndex {
    card_index("9c12bdbf-80cd-4edb-bebb-b4efcf532272")
}

/// Checklist: its entry trigger destroys one chosen artifact, including one
/// its controller owns; a neighboring creature is not a legal target.
#[test]
fn manic_vandal_can_destroy_its_controllers_artifact() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), sol_ring()])
        .hand(0, &[manic_vandal()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let ring = on_battlefield(&game, p0, sol_ring()).unwrap();
    cast_from_hand(&mut game, p0, manic_vandal());
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = game.pending().clone()
    else {
        unreachable!("the artifact trigger asks for a target")
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert_eq!(options, vec![ring]);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![ring],
            players: vec![],
        },
    )
    .expect("the controller's artifact is a legal target");
    pass_until(&mut game, stack_is_empty);

    assert!(on_battlefield(&game, p0, sol_ring()).is_none());
    assert!(in_graveyard(&game, p0, sol_ring()).is_some());
    assert!(on_battlefield(&game, p0, manic_vandal()).is_some());
}

/// With no artifact in play, the trigger has no legal target and cannot
/// destroy a creature or make its source leave the battlefield.
#[test]
fn manic_vandal_does_not_destroy_a_creature_when_no_artifact_exists() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[manic_vandal()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    cast_from_hand(&mut game, p0, manic_vandal());
    pass_until(&mut game, stack_is_empty);
    assert!(on_battlefield(&game, p0, manic_vandal()).is_some());
    assert!(on_battlefield(&game, p0, llanowar_elves()).is_some());
    assert!(in_graveyard(&game, p0, llanowar_elves()).is_none());
}
