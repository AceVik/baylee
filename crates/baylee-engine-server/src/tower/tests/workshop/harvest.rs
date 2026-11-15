#[allow(clippy::wildcard_imports)]
use super::*;

// Native manifests forbid injected opening hands. Choose a bounded synthetic
// shuffle whose ordinary opening hand can actually play this four-land story.
fn harvest_game(rarity: Rarity) -> Game {
    let mut preset = setup(
        &[
            item(Family::HarvestWheel, rarity),
            item(Family::Grove, Rarity::Common),
        ],
        &[("Plains", 30), ("Wrath of God", 5), ("Benalish Hero", 5)],
    );
    let indices = ["Plains", "Wrath of God", "Benalish Hero"]
        .map(|name| baylee_cards::decks::by_name(name).unwrap());
    for seed in 0..64 {
        preset.seed = seed;
        let mut game = Game::with_detection(&preset, Registry::default()).unwrap();
        for seat in [0, 1] {
            game.apply(PlayerId::new(seat), PlayerAction::MulliganKeep)
                .unwrap();
        }
        let hand = game
            .state()
            .zones
            .list(ZoneLocation::Hand(game.state().turn.active));
        let counts = indices.map(|index| {
            hand.iter()
                .filter(|id| {
                    game.state()
                        .object(**id)
                        .unwrap()
                        .card
                        .is_some_and(|card| card.index == index)
                })
                .count()
        });
        if counts[0] >= 4 && counts[1] >= 1 && counts[2] >= 1 {
            return game;
        }
    }
    panic!("bounded shuffle fixture must contain four lands, the creature and removal");
}

#[test]
fn exiling_a_token_does_not_charge_the_harvest_wheel() {
    let mut game = prepared(
        &[
            item(Family::HarvestWheel, Rarity::MythicRare),
            item(Family::Grove, Rarity::Common),
        ],
        &[("Plains", 30), ("Swords to Plowshares", 10)],
    );
    let player = game.state().turn.active;
    main(&mut game, 1, player);
    land(&mut game, player);
    let token = creatures(game.state(), player, true)[0];
    let wheel = source(&game, player, GROWTH_RELICS[2][3]);
    cast(&mut game, player, "Swords to Plowshares", 1, &[token]);
    resolve(&mut game);
    assert!(creatures(game.state(), player, true).is_empty());
    assert_eq!(charges(&game, wheel), 0);
}

#[test]
fn harvest_tiers_count_only_own_token_deaths_and_pay_their_costs() {
    for (tier, rarity) in Rarity::ALL.into_iter().enumerate() {
        let mut game = harvest_game(rarity);
        let player = game.state().turn.active;
        let opponent = PlayerId::new(1 - player.get());
        let wheel = source(&game, player, GROWTH_RELICS[2][tier]);
        let other = source(&game, opponent, GROWTH_RELICS[2][tier]);
        main(&mut game, 1, player);
        land(&mut game, player);
        cast(&mut game, player, "Benalish Hero", 1, &[]);
        resolve(&mut game);
        for turn in [3, 5, 7] {
            main(&mut game, turn, player);
            land(&mut game, player);
        }
        assert_eq!(
            charges(&game, wheel),
            0,
            "entries do not charge a death artifact"
        );
        let mine = u16::try_from(creatures(game.state(), player, true).len()).unwrap();
        let theirs = u16::try_from(creatures(game.state(), opponent, true).len()).unwrap();
        assert_eq!(mine, 4);
        assert_eq!(creatures(game.state(), player, false).len(), 1);
        cast(&mut game, player, "Wrath of God", 4, &[]);
        resolve(&mut game);
        assert_eq!(
            charges(&game, wheel),
            mine,
            "the actual creature card is excluded"
        );
        assert_eq!(charges(&game, other), theirs);
        assert!(creatures(game.state(), player, true).is_empty());
        let price = [3, 2, 1, 1][tier];
        let uses = if tier == 3 { mine } else { 1 };
        for used in 1..=uses {
            game.apply(
                player,
                PlayerAction::ActivateAbility {
                    source: wheel,
                    ability_index: 1,
                },
            )
            .unwrap();
            assert_eq!(charges(&game, wheel), mine - price * used);
            assert_eq!(
                game.state().players[player.get() as usize]
                    .mana_pool
                    .total(),
                u64::from(used) * if tier == 3 { 3 } else { 2 }
            );
            assert!(game.state().zones.list(ZoneLocation::Stack).is_empty());
        }
        assert!(
            game.apply(
                player,
                PlayerAction::ActivateAbility {
                    source: wheel,
                    ability_index: 1
                }
            )
            .is_err()
        );
    }
}
