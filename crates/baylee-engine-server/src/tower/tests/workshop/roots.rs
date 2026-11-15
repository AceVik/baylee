#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::mana::ManaColor;

#[test]
fn root_tiers_count_own_token_entries_and_spend_real_charges_as_mana() {
    for (tier, rarity) in Rarity::ALL.into_iter().enumerate() {
        let mut game = prepared(
            &[
                item(Family::RootReservoir, rarity),
                item(Family::Grove, Rarity::MythicRare),
            ],
            &[("Forest", 30), ("Llanowar Elves", 10)],
        );
        let player = game.state().turn.active;
        let opponent = PlayerId::new(1 - player.get());
        let root = source(&game, player, GROWTH_RELICS[3][tier]);
        let other = source(&game, opponent, GROWTH_RELICS[3][tier]);
        assert_eq!(
            charges(&game, root),
            0,
            "artifacts do not add entry charges"
        );
        let activate = PlayerAction::ActivateAbility {
            source: root,
            ability_index: 1,
        };
        assert!(
            game.apply(player, activate.clone()).is_err(),
            "cannot spend absent counters"
        );
        main(&mut game, 1, player);
        assert_eq!(charges(&game, root), 4);
        assert_eq!(charges(&game, other), 0);
        land(&mut game, player);
        cast(&mut game, player, "Llanowar Elves", 1, &[]);
        resolve(&mut game);
        assert_eq!(
            charges(&game, root),
            4,
            "nontoken entry contributes nothing"
        );
        let price = [3, 2, 2, 1][tier];
        let uses = if tier < 2 { 1 } else { 4 / price };
        for used in 1..=uses {
            game.apply(player, activate.clone()).unwrap();
            assert!(
                game.state().zones.list(ZoneLocation::Stack).is_empty(),
                "mana ability resolves immediately"
            );
            assert_eq!(charges(&game, root), 4 - price * used);
            assert_eq!(
                game.state().players[player.get() as usize]
                    .mana_pool
                    .available(ManaColor::Green),
                u32::from(used)
            );
        }
        assert!(
            game.apply(player, activate).is_err(),
            "tap or charge price still applies"
        );
        let remaining = charges(&game, root);
        main(&mut game, 2, opponent);
        assert_eq!(
            charges(&game, root),
            remaining,
            "opposing tokens do not charge this source"
        );
        assert_eq!(charges(&game, other), 4);
    }
}
