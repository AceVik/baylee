//! Oracle scenarios for Agonizing Syphon.
#[allow(clippy::wildcard_imports)]
use super::*;
fn agonizing_syphon() -> CardIndex {
    card_index("11669fe5-9298-4e1a-a4f7-6a4598dbc8dd")
}

// Checklist: any target includes either player; exactly 3 damage and exactly
// 3 life gain both happen on resolution.
#[test]
fn agonizing_syphon_can_target_a_player_and_gains_life_for_its_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[agonizing_syphon(), agonizing_syphon()])
        .battlefield(1, &[fangren_hunter()])
        .life(0, 12)
        .life(1, 20)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, agonizing_syphon());
    let Pending::ChooseTargets { player_options, .. } = e.pending().clone() else {
        panic!("any target")
    };
    assert!(player_options.contains(&p0) && player_options.contains(&p1));
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 15);
    assert_eq!(e.state().players[1].life, 17);

    let creature = on_battlefield(&e, p1, fangren_hunter()).unwrap();
    cast_from_hand(&mut e, p0, agonizing_syphon());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending().clone()
    else {
        panic!("any target includes creatures and players")
    };
    assert!(options.contains(&creature));
    assert!(player_options.contains(&p0) && player_options.contains(&p1));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![creature],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(creature).unwrap().damage, 3);
    assert_eq!(
        e.state().players[0].life,
        18,
        "life gain also happens on creature damage"
    );
    assert_eq!(e.state().players[1].life, 17);
}
