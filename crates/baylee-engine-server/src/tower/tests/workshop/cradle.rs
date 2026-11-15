#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_engine::choice::YesNoPrompt;

#[test]
fn each_cradle_tier_is_optional_costs_real_mana_and_counts_one_gain_event() {
    for (tier, rarity) in Rarity::ALL.into_iter().enumerate() {
        let mut game = prepared(
            &[
                item(Family::SeedCradle, rarity),
                item(Family::SpringChalice, Rarity::MythicRare),
            ],
            &[("Plains", 40)],
        );
        let player = game.state().turn.active;
        let opponent = PlayerId::new(1 - player.get());
        until(&mut game, |game| {
            matches!(game.pending(), Pending::YesNo { .. })
        });
        assert_eq!(game.pending().asked(), Some(player));
        assert_eq!(game.state().players[player.get() as usize].life, 25);
        game.apply(player, PlayerAction::YesNo(false)).unwrap();
        main(&mut game, 1, player);
        assert!(creatures(game.state(), player, true).is_empty());
        for turn in [1, 3, 5] {
            main(&mut game, turn, player);
            land(&mut game, player);
        }
        until(&mut game, |game| {
            game.state().turn.number == 7 && matches!(game.pending(), Pending::YesNo { .. })
        });
        let cost = [3, 2, 1, 1][tier];
        assert!(
            matches!(game.pending(), Pending::YesNo { player: asked, prompt: YesNoPrompt::PayTax { mana }, .. } if *asked == player && usize::from(*mana) == cost)
        );
        assert_eq!(
            game.state().players[player.get() as usize]
                .mana_pool
                .total(),
            0
        );
        game.apply(player, PlayerAction::YesNo(true)).unwrap();
        mana(&mut game, player, cost);
        game.apply(player, PlayerAction::PassPriority).unwrap();
        resolve(&mut game);
        assert_eq!(
            creatures(game.state(), player, true).len(),
            if tier == 3 { 2 } else { 1 }
        );
        assert!(creatures(game.state(), opponent, true).is_empty());
        assert_eq!(
            game.state().players[player.get() as usize]
                .mana_pool
                .total(),
            0
        );
        assert_eq!(
            game.state().players[player.get() as usize].life,
            40,
            "four gains of five life, not twenty paid offers"
        );
    }
}
