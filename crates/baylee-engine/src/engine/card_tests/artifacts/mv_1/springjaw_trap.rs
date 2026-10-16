//! Oracle checklist: Flash allows the cast on an opponent's turn; activation pays {4}, tap, sacrifice up front; any-target damage resolves as 3 to a player.
#[allow(clippy::wildcard_imports)]
use super::*;

fn springjaw_trap() -> CardIndex {
    card_index("3a635b73-692f-4345-9628-cb4f9f0cddad")
}

fn jace() -> CardIndex {
    jace_the_mind_sculptor()
}

#[test]
fn trap_cannot_activate_while_tapped_even_with_target_and_mana() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[springjaw_trap()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let trap = on_battlefield(&e, p0, springjaw_trap()).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.state.set_tapped(trap, true);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("the controller has priority")
    };
    assert!(
        legal.abilities.iter().all(|(source, _)| *source != trap),
        "a tapped Trap cannot activate even with its mana cost available"
    );
}

#[test]
fn flash_trap_can_be_cast_and_activated_during_the_opponents_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[springjaw_trap()])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 5);
    e.refresh_offer();
    cast_from_hand(&mut e, p0, springjaw_trap());
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, springjaw_trap()).is_some());
    // The active player gets priority first after a spell resolves.
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    // Casting used one generic mana; the remaining four pay the ability.
    activate(&mut e, p0, springjaw_trap(), 0);
    let Pending::ChooseTargets {
        player,
        player_options,
        min,
        max,
        ..
    } = e.pending().clone()
    else {
        panic!("the Trap asks for any target: {:?}", e.pending());
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert!(player_options.contains(&p1));
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert!(
        on_battlefield(&e, p0, springjaw_trap()).is_none(),
        "mana, tap, and sacrifice are paid after target choice"
    );
    assert_eq!(
        e.state().players[1].life,
        20,
        "the damage is not dealt at announcement"
    );
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().players[1].life, 17);
    assert!(in_graveyard(&e, p0, springjaw_trap()).is_some());
}

#[test]
fn trap_damages_creatures_and_planeswalkers_for_three() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut creature = Duel::new(SEED, forest())
        .battlefield(0, &[springjaw_trap()])
        .battlefield(1, &[fangren_hunter()])
        .start();
    keep_mulligans(&mut creature);
    assert!(walk_to_own_main(&mut creature, p0));
    let hunter = on_battlefield(&creature, p1, fangren_hunter()).unwrap();
    creature.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    creature.refresh_offer();
    activate(&mut creature, p0, springjaw_trap(), 0);
    creature
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hunter],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut creature, stack_is_empty);
    assert_eq!(creature.state().object(hunter).unwrap().damage, 3);
    assert_eq!(
        pt(&creature, hunter),
        (4, 4),
        "the 4/4 survives exactly three"
    );

    let mut walker = Duel::new(SEED, forest())
        .battlefield(0, &[springjaw_trap()])
        .battlefield(1, &[jace()])
        .start();
    keep_mulligans(&mut walker);
    assert!(walk_to_own_main(&mut walker, p0));
    let jace_id = on_battlefield(&walker, p1, jace()).unwrap();
    walker
        .dev_state_mut(p1)
        .unwrap()
        .object_mut(jace_id)
        .unwrap()
        .counters
        .set(CounterKind::Loyalty, 6);
    walker.refresh_offer();
    let loyalty_before = counters_on(&walker, jace_id, CounterKind::Loyalty);
    walker.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    walker.refresh_offer();
    activate(&mut walker, p0, springjaw_trap(), 0);
    walker
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![jace_id],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut walker, stack_is_empty);
    let loyalty_after = counters_on(&walker, jace_id, CounterKind::Loyalty);
    assert_eq!(loyalty_after, loyalty_before - 3);
}

#[test]
fn battle_target_loses_exact_defense_without_harming_bystanders() {
    use crate::engine::card_tests::battle_damage_kit as battle_kit;
    let p0 = PlayerId::new(0);
    let (mut e, battle) = battle_kit::game(6904, &[springjaw_trap()], &[]);
    battle_kit::float(&mut e, ManaColor::Red, 4);
    let source = on_battlefield(&e, p0, springjaw_trap()).unwrap();
    activate(&mut e, p0, springjaw_trap(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("any-target Battle option: {:?}", e.pending());
    };
    assert!(options.contains(&battle));
    aim_at(&mut e, p0, battle);
    assert_eq!(
        e.state().object(source).unwrap().zone,
        Zone::Graveyard,
        "the damage uses the sacrificed source's identity"
    );
    pass_until(&mut e, stack_is_empty);
    battle_kit::assert_nonlethal(&e, battle, 3);
}
