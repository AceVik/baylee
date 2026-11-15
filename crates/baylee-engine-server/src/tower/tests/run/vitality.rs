#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn heartwood_and_hearth_add_life_and_mythic_drains_on_a_real_creature() {
    let player = PlayerId::new(0);
    for family in [Family::Heartwood, Family::Hearthstone] {
        for rarity in Rarity::ALL {
            let mut game = opening(&[item(family, rarity)], &["Memnite"]);
            let life = game.state().players[0].life;
            let enemy = game.state().players[1].life;
            let heal = match family {
                Family::Heartwood => i32::from(rarity.tier() >= 3),
                _ => i32::from(rarity.tier()),
            };
            if family == Family::Heartwood {
                assert_eq!(
                    life,
                    20 + [4, 7, 11, 16][usize::from(rarity.tier() - 1)]
                        + i32::from(rarity.tier() >= 2)
                );
            }
            cast_free(&mut game, "Memnite");
            assert_eq!(game.state().players[0].life, life + heal);
            assert_eq!(
                game.state().players[1].life,
                enemy - i32::from(rarity == Rarity::MythicRare)
            );
            if family == Family::Hearthstone && rarity.tier() >= 2 {
                assert_eq!(plants(&game, player), 1);
            }
        }
    }
}

#[test]
fn coil_pays_actual_life_and_mana_draws_and_only_mythic_can_repeat() {
    let player = PlayerId::new(0);
    for rarity in Rarity::ALL {
        let definition = item(Family::LifeCoil, rarity);
        let mut game = opening(&[definition], &[]);
        for turn in [1, 3, 5] {
            main(&mut game, turn, player);
            land(&mut game);
        }
        float_mana(&mut game, 3);
        let id = source(&game, player, definition);
        let life = game.state().players[0].life;
        let enemy = game.state().players[1].life;
        let hand = game.state().zones.list(ZoneLocation::Hand(player)).len();
        let action = PlayerAction::ActivateAbility {
            source: id,
            ability_index: 0,
        };
        game.apply(player, action.clone()).unwrap();
        resolve(&mut game);
        assert_eq!(
            game.state().players[0].life,
            life - 2 + i32::from(rarity.tier() >= 2)
        );
        assert_eq!(
            game.state().players[1].life,
            enemy - i32::from(rarity == Rarity::MythicRare)
        );
        assert_eq!(
            game.state().zones.list(ZoneLocation::Hand(player)).len(),
            hand + 1
        );
        if rarity == Rarity::MythicRare {
            game.apply(player, action).unwrap();
            resolve(&mut game);
            assert_eq!(
                game.state().zones.list(ZoneLocation::Hand(player)).len(),
                hand + 2
            );
        } else {
            assert!(game.apply(player, action).is_err());
        }
    }
}

#[test]
fn amber_spends_two_life_event_charges_and_obeys_its_turn_cap() {
    let player = PlayerId::new(0);
    for rarity in Rarity::ALL {
        let definition = item(Family::BloodAmber, rarity);
        let mut game = prepared(
            &[definition, item(Family::SpringChalice, Rarity::Common)],
            &[],
            11,
        );
        main(&mut game, 5, player);
        let id = source(&game, player, definition);
        let count = charges(&game, id);
        assert!(count >= 3);
        let enemy = game.state().players[1].life;
        let action = PlayerAction::ActivateAbility {
            source: id,
            ability_index: 1,
        };
        game.apply(player, action.clone()).unwrap();
        game.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![PlayerId::new(1)],
            },
        )
        .unwrap();
        resolve(&mut game);
        assert_eq!(charges(&game, id), count - 2);
        assert_eq!(
            game.state().players[1].life,
            enemy - i32::from(rarity.tier())
        );
        if rarity == Rarity::MythicRare {
            game.apply(player, action).unwrap();
            game.apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: Vec::new(),
                    players: vec![PlayerId::new(1)],
                },
            )
            .unwrap();
            resolve(&mut game);
            assert_eq!(charges(&game, id), count - 4);
        } else {
            assert!(game.apply(player, action).is_err());
        }
    }
}
