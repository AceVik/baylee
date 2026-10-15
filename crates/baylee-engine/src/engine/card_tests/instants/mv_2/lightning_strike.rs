//! Oracle: deal 3 damage to any target.
#[allow(clippy::wildcard_imports)]
use super::*;
fn lightning_strike() -> CardIndex {
    card_index("f34b9bc4-7bfe-47fd-ba23-4eeeb46026eb")
}
/// Checklist: any target offers players, creatures, and planeswalkers; exact damage goes to each kind.
#[test]
fn any_target_deals_three_to_a_player_creature_or_planeswalker() {
    let (p, q) = (PlayerId::new(0), PlayerId::new(1));
    let mut g = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[fangren_hunter(), oko_thief_of_crowns()])
        .hand(
            0,
            &[lightning_strike(), lightning_strike(), lightning_strike()],
        )
        .life(1, 10)
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    let creature = on_battlefield(&g, q, fangren_hunter()).unwrap();
    let walker = on_battlefield(&g, q, oko_thief_of_crowns()).unwrap();
    let loyalty_before = counters_on(&g, walker, CounterKind::Loyalty);

    cast_from_hand(&mut g, p, lightning_strike());
    let Pending::ChooseTargets { player_options, .. } = g.pending().clone() else {
        panic!("any target")
    };
    assert!(player_options.contains(&p) && player_options.contains(&q));
    g.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![q],
        },
    )
    .unwrap();
    pass_until(&mut g, stack_is_empty);
    assert_eq!(g.state().players[1].life, 7);
    assert_eq!(g.state().players[0].life, 20);

    cast_from_hand(&mut g, p, lightning_strike());
    let Pending::ChooseTargets { options, .. } = g.pending().clone() else {
        panic!("any target includes objects")
    };
    assert!(options.contains(&creature) && options.contains(&walker));
    g.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![creature],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut g, stack_is_empty);
    assert_eq!(g.state().object(creature).unwrap().damage, 3);
    assert_eq!(
        counters_on(&g, walker, CounterKind::Loyalty),
        loyalty_before
    );

    cast_from_hand(&mut g, p, lightning_strike());
    let Pending::ChooseTargets { options, .. } = g.pending().clone() else {
        panic!("planeswalker is a legal any target")
    };
    assert!(options.contains(&walker));
    g.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![walker],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut g, stack_is_empty);
    assert_eq!(
        counters_on(&g, walker, CounterKind::Loyalty),
        loyalty_before - 3
    );
    assert_eq!(g.state().players[1].life, 7);
}
