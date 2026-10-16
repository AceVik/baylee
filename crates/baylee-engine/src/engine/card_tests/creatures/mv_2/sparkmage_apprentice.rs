// Oracle checklist: both players; own/opponent creature and planeswalker resolve; lethal boundary; exact damage/loyalty/life; noncreature exclusion; actual Battle defense loss and unchanged bystanders.
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("69003cbc-8e5a-41dc-ac53-18f6e776217b")
}

#[test]
fn any_target_hits_both_players_and_only_the_named_recipient() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for recipient in [p0, p1] {
        let mut game = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[mountain(), mountain(), fangren_hunter(), quiet_artifact()],
            )
            .battlefield(1, &[fangren_hunter()])
            .hand(0, &[subject()])
            .start();
        keep_mulligans(&mut game);
        reach_main_phase(&mut game, p0);
        let life = [game.state().players[0].life, game.state().players[1].life];
        cast_from_hand(&mut game, p0, subject());
        pass_until(&mut game, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });

        let Pending::ChooseTargets {
            player_options,
            options,
            min,
            max,
            ..
        } = game.pending().clone()
        else {
            panic!("any target menu")
        };
        assert_eq!((min, max), (1, 1));
        assert!(player_options.contains(&p0));
        assert!(player_options.contains(&p1));
        assert!(!options.contains(&on_battlefield(&game, p0, quiet_artifact()).unwrap()));
        game.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![recipient],
            },
        )
        .unwrap();
        pass_until(&mut game, stack_is_empty);
        for seat in [p0, p1] {
            let index = usize::from(seat != p0);
            assert_eq!(
                game.state().players[index].life,
                life[index] - i32::from(seat == recipient)
            );
            assert_eq!(
                game.state()
                    .object(on_battlefield(&game, seat, fangren_hunter()).unwrap())
                    .unwrap()
                    .damage,
                0
            );
        }
    }
}

#[test]
fn any_target_creatures_and_planeswalkers_resolve_exact_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for seat in [p0, p1] {
        for card in [fangren_hunter(), llanowar_elves(), jace_the_mind_sculptor()] {
            let mut game = Duel::new(SEED, forest())
                .battlefield(0, &[mountain(), mountain(), quiet_artifact()])
                .battlefield(
                    usize::from(seat != p0),
                    &[card, quiet_artifact(), mountain(), mountain()],
                )
                .hand(0, &[subject()])
                .start();
            keep_mulligans(&mut game);
            reach_main_phase(&mut game, p0);
            let target = on_battlefield(&game, seat, card).unwrap();
            let loyalty = game
                .state()
                .object(target)
                .unwrap()
                .counters
                .get(CounterKind::Loyalty);
            cast_from_hand(&mut game, p0, subject());
            pass_until(&mut game, |e| {
                matches!(e.pending(), Pending::ChooseTargets { .. })
            });

            aim_at(&mut game, p0, target);
            pass_until(&mut game, stack_is_empty);
            if card == jace_the_mind_sculptor() {
                if loyalty <= 1 {
                    assert!(in_graveyard(&game, seat, card).is_some());
                } else {
                    assert_eq!(
                        game.state()
                            .object(target)
                            .unwrap()
                            .counters
                            .get(CounterKind::Loyalty),
                        loyalty - 1
                    );
                }
            } else if card == llanowar_elves() {
                assert!(in_graveyard(&game, seat, card).is_some());
            } else {
                assert_eq!(game.state().object(target).unwrap().damage, 1);
                assert_eq!(pt(&game, target), (4, 4));
            }
            assert!(on_battlefield(&game, seat, quiet_artifact()).is_some());
        }
    }
}

#[test]
fn battle_target_loses_exact_defense_without_harming_bystanders() {
    use crate::engine::card_tests::battle_damage_kit as battle_kit;
    let p0 = PlayerId::new(0);
    let (mut e, battle) = battle_kit::game(6902, &[], &[subject()]);
    battle_kit::float(&mut e, ManaColor::Red, 2);
    cast_with_floating(&mut e, p0, subject());
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
