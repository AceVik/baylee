#[allow(clippy::wildcard_imports)]
use super::*;

fn aegis_card() -> CardIndex {
    card_index("f8e1b38a-2a77-4cf1-8bef-31acb61ede10")
}

/// Checklist: one creature target on either side; asymmetric +1/+7; unrelated
/// body unchanged; modifier expires at cleanup.
#[test]
fn aegis_of_the_heavens_applies_exact_asymmetric_modifier_until_cleanup() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[aegis_card()])
        .battlefield(1, &[fangren_hunter(), wild_elephant()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let target = on_battlefield(&game, p1, fangren_hunter()).unwrap();
    let bystander = on_battlefield(&game, p1, wild_elephant()).unwrap();
    cast_from_hand(&mut game, p0, aegis_card());
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
    assert_eq!(pt(&game, target), (5, 11));
    assert_eq!(pt(&game, bystander), (3, 3));
    pass_until(&mut game, |e| {
        e.state().turn.active == p1 && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(pt(&game, target), (4, 4));
}
