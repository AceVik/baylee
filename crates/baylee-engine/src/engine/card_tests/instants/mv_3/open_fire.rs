// Oracle checklist: both players; own/opponent creature and planeswalker resolve; lethal boundary; exact damage/loyalty/life; noncreature exclusion; battle legal offer only (known defense-resolution gap).
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("8b218023-50b8-4d9d-b17b-9647479d3675")
}

#[test]
fn any_target_hits_both_players_and_only_the_named_recipient() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for recipient in [p0, p1] {
        let mut game = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    mountain(),
                    mountain(),
                    mountain(),
                    fangren_hunter(),
                    quiet_artifact(),
                ],
            )
            .battlefield(1, &[fangren_hunter()])
            .hand(0, &[subject()])
            .start();
        keep_mulligans(&mut game);
        reach_main_phase(&mut game, p0);
        let life = [game.state().players[0].life, game.state().players[1].life];
        cast_from_hand(&mut game, p0, subject());

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
                life[index] - if seat == recipient { 3 } else { 0 }
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
        for card in [fangren_hunter(), wild_elephant(), jace_the_mind_sculptor()] {
            let mut game = Duel::new(SEED, forest())
                .battlefield(0, &[mountain(), mountain(), mountain(), quiet_artifact()])
                .battlefield(
                    usize::from(seat != p0),
                    &[card, quiet_artifact(), mountain(), mountain(), mountain()],
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

            aim_at(&mut game, p0, target);
            pass_until(&mut game, stack_is_empty);
            if card == jace_the_mind_sculptor() {
                if loyalty <= 3 {
                    assert!(in_graveyard(&game, seat, card).is_some());
                } else {
                    assert_eq!(
                        game.state()
                            .object(target)
                            .unwrap()
                            .counters
                            .get(CounterKind::Loyalty),
                        loyalty - 3
                    );
                }
            } else if card == wild_elephant() {
                assert!(in_graveyard(&game, seat, card).is_some());
            } else {
                assert_eq!(game.state().object(target).unwrap().damage, 3);
                assert_eq!(pt(&game, target), (4, 4));
            }
            assert!(on_battlefield(&game, seat, quiet_artifact()).is_some());
        }
    }
}

// Known engine gap: battles are legal targets, but defense counters/damage
// resolution are not implemented. This test proves the legal offer only.
#[test]
fn battle_is_offered_as_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let battle = baylee_core::generated::index::INVASION_OF_IKORIA;
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[battle])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let target = on_battlefield(&game, p1, battle).unwrap();
    cast_from_hand(&mut game, p0, subject());

    let Pending::ChooseTargets { options, .. } = game.pending() else {
        panic!("target menu")
    };
    assert!(options.contains(&target));
}
