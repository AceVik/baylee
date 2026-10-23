#[allow(clippy::wildcard_imports)]
use super::*;

fn tethered_skirge() -> CardIndex {
    card_index("42bbef66-967b-4922-8b2e-e57d1b8244bb")
}

#[test]
fn tethered_skirge_loses_life_when_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), tethered_skirge()])
        .hand(0, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let skirge = on_battlefield(&engine, p0, tethered_skirge()).unwrap();
    cast_from_hand(&mut engine, p0, giant_growth());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![skirge],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(pt(&engine, skirge), (5, 5));
}
