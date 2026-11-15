#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::mana::ManaColor;

#[test]
fn upgraded_satchel_loots_once_and_mythic_heals_for_the_draw() {
    let player = PlayerId::new(0);
    for rarity in [Rarity::Rare, Rarity::MythicRare] {
        let definition = item(Family::Satchel, rarity);
        let mut game = opening(&[definition], &[]);
        land(&mut game);
        main(&mut game, 3, player);
        land(&mut game);
        float_mana(&mut game, 2);
        let life = game.state().players[0].life;
        let hand = game.state().zones.list(ZoneLocation::Hand(player)).len();
        let id = source(&game, player, definition);
        let action = PlayerAction::ActivateAbility {
            source: id,
            ability_index: 1,
        };
        game.apply(player, action.clone()).unwrap();
        resolve(&mut game);
        assert_eq!(
            game.state().zones.list(ZoneLocation::Hand(player)).len(),
            hand
        );
        assert_eq!(
            game.state().players[0].life,
            life + i32::from(rarity == Rarity::MythicRare)
        );
        assert!(game.apply(player, action).is_err());
    }
}

#[test]
fn compass_filters_once_and_mythic_draws_only_for_the_first_spell() {
    let player = PlayerId::new(0);
    for rarity in [Rarity::Rare, Rarity::MythicRare] {
        let definition = item(Family::Compass, rarity);
        let mut game = opening(&[definition], &["Lotus Petal", "Spellbook"]);
        land(&mut game);
        float_mana(&mut game, 1);
        let id = source(&game, player, definition);
        let action = PlayerAction::ActivateAbility {
            source: id,
            ability_index: 1,
        };
        game.apply(player, action.clone()).unwrap();
        game.apply(player, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
        assert_eq!(
            game.state().players[0].mana_pool.available(ManaColor::Blue),
            1
        );
        assert!(game.apply(player, action).is_err());
        let hand = game.state().zones.list(ZoneLocation::Hand(player)).len();
        cast_free(&mut game, "Lotus Petal");
        cast_free(&mut game, "Spellbook");
        assert_eq!(
            game.state().zones.list(ZoneLocation::Hand(player)).len(),
            hand - 2 + usize::from(rarity == Rarity::MythicRare)
        );
    }
}
