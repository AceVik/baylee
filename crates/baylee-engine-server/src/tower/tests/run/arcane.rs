#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn spindle_uses_charges_color_choice_and_a_real_per_turn_activation_limit() {
    use baylee_core::mana::ManaColor;
    let player = PlayerId::new(0);
    for rarity in Rarity::ALL {
        let definition = item(Family::StormSpindle, rarity);
        let mut game = opening(&[definition], &["Lotus Petal", "Welding Jar", "Spellbook"]);
        for name in ["Lotus Petal", "Welding Jar", "Spellbook"] {
            cast_free(&mut game, name);
        }
        let id = source(&game, player, definition);
        let before = charges(&game, id);
        let cost = [3, 2, 1, 1][usize::from(rarity.tier() - 1)];
        let action = PlayerAction::ActivateAbility {
            source: id,
            ability_index: 1,
        };
        game.apply(player, action.clone()).unwrap();
        assert!(matches!(game.pending(), Pending::ChooseColor { .. }));
        game.apply(player, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
        assert_eq!(charges(&game, id), before - cost);
        assert_eq!(game.state().players[0].mana_pool.total(), 1);
        assert!(
            game.state().zones.list(ZoneLocation::Stack).is_empty(),
            "ordinary mana ability"
        );
        if rarity == Rarity::MythicRare {
            game.apply(player, action).unwrap();
            game.apply(player, PlayerAction::ChooseColor(ManaColor::Blue))
                .unwrap();
            assert_eq!(game.state().players[0].mana_pool.total(), 2);
        } else {
            assert!(
                game.apply(player, action).is_err(),
                "turn cap is paid-action admission"
            );
        }
    }
}

#[test]
fn lantern_tiers_apply_spell_drain_and_unlock_their_life_and_token_synergies() {
    let player = PlayerId::new(0);
    for rarity in Rarity::ALL {
        let mut game = opening(&[item(Family::EmberLantern, rarity)], &["Lotus Petal"]);
        let life = game.state().players[0].life;
        cast_free(&mut game, "Lotus Petal");
        assert_eq!(game.state().players[1].life, 20 - i32::from(rarity.tier()));
        assert_eq!(
            game.state().players[0].life,
            life + i32::from(rarity.tier() >= 3)
        );
        assert_eq!(
            plants(&game, player),
            usize::from(rarity == Rarity::MythicRare)
        );
    }
}

#[test]
fn lens_spends_real_draw_charges_and_asks_for_the_tiered_scry() {
    let player = PlayerId::new(0);
    for rarity in Rarity::ALL {
        let definition = item(Family::TideglassLens, rarity);
        let mut game = prepared(&[definition], &[], 19);
        let id = source(&game, player, definition);
        main(&mut game, 7, player);
        let stock = charges(&game, id);
        assert!(stock >= 3);
        game.apply(
            player,
            PlayerAction::ActivateAbility {
                source: id,
                ability_index: 1,
            },
        )
        .unwrap();
        assert_eq!(charges(&game, id), stock - 3);
        until(&mut game, |game| {
            matches!(game.pending(), Pending::Arrange { .. })
        });
        let Pending::Arrange { cards, .. } = game.pending() else {
            unreachable!()
        };
        assert_eq!(cards.len(), usize::from(rarity.tier()));
        advance(&mut game);
        resolve(&mut game);
        assert_eq!(
            game.state().arena.get(id).unwrap().zone,
            baylee_engine::zone::Zone::Command
        );
    }
}
