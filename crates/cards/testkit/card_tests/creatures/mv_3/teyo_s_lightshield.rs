//! Oracle: when this enters, put a +1/+1 counter on target creature you control.
#[allow(clippy::wildcard_imports)]
use super::*;
fn teyo_s_lightshield() -> CardIndex {
    card_index("f1305d49-e543-49e9-9b36-e56f93f63bc3")
}
/// Checklist: own creature target gets one +1/+1 counter; opponent's creature is excluded; source body unaffected.
#[test]
fn counter_is_put_on_a_target_creature_you_control() {
    let (p, q) = (PlayerId::new(0), PlayerId::new(1));
    let mut g = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .battlefield(1, &[wild_elephant()])
        .hand(0, &[teyo_s_lightshield()])
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    let own = on_battlefield(&g, p, llanowar_elves()).unwrap();
    let foe = on_battlefield(&g, q, wild_elephant()).unwrap();
    cast_from_hand(&mut g, p, teyo_s_lightshield());
    pass_until(&mut g, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = g.pending().clone() else {
        unreachable!()
    };
    let source = on_battlefield(&g, p, teyo_s_lightshield()).unwrap();
    assert!(
        options.contains(&own),
        "the controlled Elf is legal: {options:?}"
    );
    assert!(
        options.contains(&source),
        "the source creature is also controlled: {options:?}"
    );
    assert!(
        !options.contains(&foe),
        "the opponent creature is excluded: {options:?}"
    );
    g.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![own],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut g, stack_is_empty);
    assert_eq!(pt(&g, own), (2, 2));
    assert_eq!(counters_on(&g, own, CounterKind::P1P1), 1);
    assert_eq!(pt(&g, foe), (3, 3));
    assert_eq!(
        pt(&g, on_battlefield(&g, p, teyo_s_lightshield()).unwrap()),
        (0, 3)
    );
}
