//! Oracle: {3}, {T}, Sacrifice this artifact: It deals 2 damage to any target.
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;
use baylee_core::generated::index;

fn subject() -> CardIndex {
    card_index("ab8301da-f9f7-4d90-afae-bd7e60834c22")
}

// Checklist: exact mana, tap and self-sacrifice costs; own/opposing creature
// targets, noncreature exclusions, exact damage and unchanged bystanders.
#[test]
fn damage_and_sacrifice_cost_are_exact() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for target_own in [false, true] {
        let mut board = vec![subject(), index::HILL_GIANT];
        board.extend(vec![island(); 3]);
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .battlefield(1, &[index::HILL_GIANT, sol_ring()])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let own = on_battlefield(&e, p0, index::HILL_GIANT).unwrap();
        let foe = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
        let artifact = on_battlefield(&e, p1, sol_ring()).unwrap();
        tap_all_mana(&mut e, p0);
        activate(&mut e, p0, subject(), 0);
        let Pending::ChooseTargets {
            options,
            player_options,
            ..
        } = e.pending()
        else {
            panic!("target prompt")
        };
        assert!(options.contains(&own));
        assert!(options.contains(&foe));
        assert!(!options.contains(&artifact));
        assert!(player_options.contains(&p0));
        assert!(player_options.contains(&p1));
        let target = if target_own { own } else { foe };
        aim_at(&mut e, p0, target);
        assert!(
            in_graveyard(&e, p0, subject()).is_some(),
            "sacrifice is paid before resolving"
        );
        assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Blue), 0);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(target).unwrap().damage, 2);
        assert_eq!(
            e.state()
                .object(if target_own { foe } else { own })
                .unwrap()
                .damage,
            0
        );
        assert!(on_battlefield(&e, p1, sol_ring()).is_some());
    }
}
#[test]
fn insufficient_mana_and_tapped_source_are_not_offered() {
    let p0 = PlayerId::new(0);
    for tapped in [false, true] {
        let mut board = vec![subject()];
        board.extend(vec![island(); if tapped { 3 } else { 2 }]);
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .battlefield(1, &[index::HILL_GIANT])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let source = on_battlefield(&e, p0, subject()).unwrap();
        if tapped {
            e.dev_state_mut(p0)
                .unwrap()
                .object_mut(source)
                .unwrap()
                .status
                .insert(Status::TAPPED);
            e.refresh_offer();
        }
        tap_all_mana(&mut e, p0);
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.iter().any(|(id, _)| *id == source));
        assert!(on_battlefield(&e, p0, subject()).is_some());
    }
}

// Checklist: actually resolve both player recipients and planeswalker damage.
// The separate real-Siege scenario covers Battle damage through resolution.
#[test]
fn any_target_includes_players_and_planeswalkers() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for category in 0..3 {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[subject(), island(), island(), island()])
            .battlefield(
                1,
                &[jace_the_mind_sculptor(), index::HILL_GIANT, sol_ring()],
            )
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let walker = on_battlefield(&e, p1, jace_the_mind_sculptor()).unwrap();
        let before = [e.state().players[0].life, e.state().players[1].life];
        let loyalty = e
            .state()
            .object(walker)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty);
        tap_all_mana(&mut e, p0);
        activate(&mut e, p0, subject(), 0);
        let Pending::ChooseTargets {
            options,
            player_options,
            ..
        } = e.pending()
        else {
            panic!("any target")
        };
        assert!(options.contains(&walker));
        assert!(player_options.contains(&p0));
        assert!(player_options.contains(&p1));
        let recipient = if category == 0 { p0 } else { p1 };
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: if category == 2 { vec![walker] } else { vec![] },
                players: if category == 2 {
                    vec![]
                } else {
                    vec![recipient]
                },
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        for (seat, life_before) in before.into_iter().enumerate() {
            assert_eq!(
                e.state().players[seat].life,
                life_before
                    - if category < 2 && seat == category {
                        2
                    } else {
                        0
                    }
            );
        }
        assert_eq!(
            e.state()
                .object(walker)
                .unwrap()
                .counters
                .get(CounterKind::Loyalty),
            loyalty - if category == 2 { 2 } else { 0 }
        );
    }
}

// Resolution boundary: cost remains paid when the only creature target leaves.
#[test]
fn target_leaving_in_response_prevents_damage_but_not_cost() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut board = vec![subject()];
    board.extend(vec![island(); 3]);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[island(), index::HILL_GIANT, index::GRAY_OGRE])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let target = on_battlefield(&e, p1, index::HILL_GIANT).unwrap();
    let bystander = on_battlefield(&e, p1, index::GRAY_OGRE).unwrap();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, subject(), 0);
    aim_at(&mut e, p0, target);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut e, p1, unsummon());
    aim_at(&mut e, p1, target);
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, p1, index::HILL_GIANT).is_some());
    assert!(in_graveyard(&e, p0, subject()).is_some());
    assert_eq!(e.state().object(bystander).unwrap().damage, 0);
}

#[test]
fn battle_target_loses_exact_defense_without_harming_bystanders() {
    use crate::engine::card_tests::battle_damage_kit as battle_kit;
    let p0 = PlayerId::new(0);
    let (mut e, battle) = battle_kit::game(6903, &[subject()], &[]);
    battle_kit::float(&mut e, ManaColor::Red, 3);
    let source = on_battlefield(&e, p0, subject()).unwrap();
    activate(&mut e, p0, subject(), 0);
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
    battle_kit::assert_nonlethal(&e, battle, 2);
}
