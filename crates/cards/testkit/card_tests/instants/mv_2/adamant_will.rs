#[allow(clippy::wildcard_imports)]
use super::*;

fn adamant_will_card() -> CardIndex {
    card_index("467e22c3-6107-40ff-afc7-5960710c970b")
}
fn doom_blade_card() -> CardIndex {
    card_index("59e7f2ae-4535-4191-98be-3e65b6b2befa")
}

/// Checklist: target creature on either side; +2/+2 and indestructible;
/// destroy fails during the duration; both additions expire at cleanup.
#[test]
fn adamant_will_pumps_and_protects_until_cleanup() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[adamant_will_card()])
        .battlefield(1, &[swamp(), swamp(), fangren_hunter()])
        .hand(1, &[doom_blade_card(), doom_blade_card()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let target = on_battlefield(&game, p1, fangren_hunter()).unwrap();
    cast_from_hand(&mut game, p0, adamant_will_card());
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("creature target choice")
    };
    assert!(options.contains(&target));
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![target],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(pt(&game, target), (6, 6));

    game.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut game, p1, doom_blade_card());
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("Doom Blade target choice")
    };
    assert!(options.contains(&target));
    game.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![target],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(
        pt(&game, target),
        (6, 6),
        "destroy cannot destroy it this turn"
    );

    pass_until(&mut game, |e| {
        e.state().turn.active == p1 && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(
        pt(&game, target),
        (4, 4),
        "pump and indestructible ended at cleanup"
    );
    cast_from_hand(&mut game, p1, doom_blade_card());
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("Doom Blade target choice after cleanup")
    };
    assert!(options.contains(&target));
    game.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![target],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert!(on_battlefield(&game, p1, fangren_hunter()).is_none());
    assert!(in_graveyard(&game, p1, fangren_hunter()).is_some());
}
