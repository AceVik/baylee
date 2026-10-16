//! Oracle: when this enters, it deals 1 damage to any target.
#[allow(clippy::wildcard_imports)]
use super::*;
fn blisterstick_shaman() -> CardIndex {
    card_index("6f76fec2-6fd0-4d33-b39a-f1684922d478")
}
/// Checklist: any target includes either player and creature; choose opponent for one damage.
#[test]
fn entry_trigger_can_deal_one_to_a_player() {
    let (p, q) = (PlayerId::new(0), PlayerId::new(1));
    let mut g = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[blisterstick_shaman()])
        .life(1, 8)
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    cast_from_hand(&mut g, p, blisterstick_shaman());
    pass_until(&mut g, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { player_options, .. } = g.pending().clone() else {
        unreachable!()
    };
    assert!(player_options.contains(&q));
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
    assert!(on_battlefield(&g, p, blisterstick_shaman()).is_some());
}

/// Creature and planeswalker objects are both legal any-target recipients.
#[test]
fn entry_trigger_can_damage_a_creature_or_reduce_planeswalker_loyalty() {
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
        .hand(0, &[blisterstick_shaman(), blisterstick_shaman()])
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    let creature = on_battlefield(&g, q, fangren_hunter()).unwrap();
    let walker = on_battlefield(&g, q, oko_thief_of_crowns()).unwrap();
    let loyalty_before = counters_on(&g, walker, CounterKind::Loyalty);

    cast_from_hand(&mut g, p, blisterstick_shaman());
    pass_until(&mut g, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = g.pending().clone()
    else {
        unreachable!()
    };
    assert!(options.contains(&creature) && options.contains(&walker));
    assert!(player_options.contains(&p) && player_options.contains(&q));
    g.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![creature],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut g, |e| {
        e.state().object(creature).is_some_and(|o| o.damage == 1)
    });
    assert_eq!(g.state().object(creature).unwrap().damage, 1);
    pass_until(&mut g, stack_is_empty);

    cast_from_hand(&mut g, p, blisterstick_shaman());
    pass_until(&mut g, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = g.pending().clone() else {
        unreachable!()
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
        loyalty_before - 1
    );
    assert_eq!(g.state().object(creature).unwrap().damage, 1);
}

#[test]
fn battle_target_loses_exact_defense_without_harming_bystanders() {
    use crate::engine::card_tests::battle_damage_kit as battle_kit;
    let p0 = PlayerId::new(0);
    let (mut e, battle) = battle_kit::game(6901, &[], &[blisterstick_shaman()]);
    battle_kit::float(&mut e, ManaColor::Red, 3);
    cast_with_floating(&mut e, p0, blisterstick_shaman());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("any-target Battle option: {:?}", e.pending());
    };
    assert!(options.contains(&battle));
    aim_at(&mut e, p0, battle);
    pass_until(&mut e, stack_is_empty);
    battle_kit::assert_nonlethal(&e, battle, 1);
}
