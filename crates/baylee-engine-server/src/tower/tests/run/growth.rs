#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::mana::ManaColor;

#[test]
fn root_tiers_pay_their_charge_cost_and_lose_the_turn_limit_only_at_rare() {
    let player = PlayerId::new(0);
    for rarity in Rarity::ALL {
        let definition = item(Family::RootReservoir, rarity);
        let mut game = prepared(
            &[definition, item(Family::Grove, Rarity::MythicRare)],
            &[],
            19,
        );
        main(&mut game, 3, player);
        let id = source(&game, player, definition);
        let before = charges(&game, id);
        let cost = [3, 2, 2, 1][usize::from(rarity.tier() - 1)];
        assert!(before >= 2 * cost);
        let action = PlayerAction::ActivateAbility {
            source: id,
            ability_index: 1,
        };
        game.apply(player, action.clone()).unwrap();
        assert_eq!(charges(&game, id), before - cost);
        assert_eq!(
            game.state().players[0]
                .mana_pool
                .available(ManaColor::Green),
            1
        );
        if rarity.tier() >= 3 {
            game.apply(player, action).unwrap();
            assert_eq!(charges(&game, id), before - 2 * cost);
            assert_eq!(
                game.state().players[0]
                    .mana_pool
                    .available(ManaColor::Green),
                2
            );
        } else {
            assert!(game.apply(player, action).is_err());
        }
    }
}

#[test]
fn harvest_stores_actual_token_deaths_and_survives_the_board_wipe() {
    let player = PlayerId::new(0);
    for rarity in Rarity::ALL {
        let definition = item(Family::HarvestWheel, rarity);
        let mut game = opening(
            &[definition, item(Family::Grove, Rarity::MythicRare)],
            &["Powder Keg"],
        );
        land(&mut game);
        main(&mut game, 3, player);
        land(&mut game);
        float_mana(&mut game, 2);
        cast_free(&mut game, "Powder Keg");
        let before = plants(&game, player);
        let id = source(&game, player, definition);
        assert_eq!(charges(&game, id), 0);
        let keg = permanent(&game, "Powder Keg");
        game.apply(
            player,
            PlayerAction::ActivateAbility {
                source: keg,
                ability_index: 1,
            },
        )
        .unwrap();
        resolve(&mut game);
        assert_eq!(plants(&game, player), 0);
        assert_eq!(usize::from(charges(&game, id)), before);
        assert_eq!(
            game.state().arena.get(id).unwrap().zone,
            baylee_engine::zone::Zone::Command
        );
        let cost = [3, 2, 1, 1][usize::from(rarity.tier() - 1)];
        let action = PlayerAction::ActivateAbility {
            source: id,
            ability_index: 1,
        };
        game.apply(player, action.clone()).unwrap();
        assert_eq!(usize::from(charges(&game, id)), before - cost);
        let mana = if rarity == Rarity::MythicRare { 3 } else { 2 };
        assert_eq!(
            game.state().players[0]
                .mana_pool
                .available(ManaColor::Colorless),
            mana
        );
        if rarity == Rarity::MythicRare {
            game.apply(player, action).unwrap();
            assert_eq!(
                game.state().players[0]
                    .mana_pool
                    .available(ManaColor::Colorless),
                6
            );
        } else {
            assert!(game.apply(player, action).is_err());
        }
    }
}

#[test]
fn grave_bell_observes_real_nontoken_death_and_mythic_draws_a_card() {
    let player = PlayerId::new(0);
    for rarity in Rarity::ALL {
        let mut game = opening(&[item(Family::GraveBell, rarity)], &["Sakura-Tribe Elder"]);
        land(&mut game);
        main(&mut game, 3, player);
        land(&mut game);
        float_mana(&mut game, 2);
        cast_free(&mut game, "Sakura-Tribe Elder");
        let elder = permanent(&game, "Sakura-Tribe Elder");
        let life = game.state().players[0].life;
        let hand = game.state().zones.list(ZoneLocation::Hand(player)).len();
        game.apply(
            player,
            PlayerAction::ActivateAbility {
                source: elder,
                ability_index: 0,
            },
        )
        .unwrap();
        resolve(&mut game);
        assert_eq!(game.state().players[1].life, 20 - i32::from(rarity.tier()));
        assert_eq!(
            game.state().players[0].life,
            life + i32::from(rarity.tier() >= 2)
        );
        assert_eq!(
            game.state().zones.list(ZoneLocation::Hand(player)).len(),
            hand + usize::from(rarity == Rarity::MythicRare)
        );
    }
}
