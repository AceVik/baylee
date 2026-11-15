#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::KeywordSet;
use baylee_core::ids::Defender;

#[test]
fn mythic_banner_tramples_over_a_real_blocker() {
    let mut game = prepared(
        &[
            item(Family::SwarmBanner, Rarity::MythicRare),
            item(Family::Grove, Rarity::Common),
        ],
        &[("Plains", 30), ("Disenchant", 10)],
    );
    let player = game.state().turn.active;
    let opponent = PlayerId::new(1 - player.get());
    for turn in [1, 3] {
        main(&mut game, turn, player);
        land(&mut game, player);
    }
    let theirs = source(&game, opponent, GROWTH_RELICS[1][3]);
    cast(&mut game, player, "Disenchant", 2, &[theirs]);
    resolve(&mut game);
    until(&mut game, |game| {
        matches!(game.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = game.pending() else {
        unreachable!()
    };
    let attacker = attackers[0];
    let blocker = creatures(game.state(), opponent, true)[0];
    game.apply(
        player,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(opponent))],
        },
    )
    .unwrap();
    until(&mut game, |game| {
        matches!(game.pending(), Pending::ChooseBlockers { .. })
    });
    game.apply(
        opponent,
        PlayerAction::DeclareBlockers {
            blockers: vec![(blocker, attacker)],
        },
    )
    .unwrap();
    until(&mut game, |game| game.state().turn.step == Step::CombatEnd);
    assert_eq!(game.state().players[opponent.get() as usize].life, 18);
    assert!(
        !game
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&blocker)
    );
    assert!(
        game.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&attacker)
    );
}

#[test]
fn banner_tiers_change_token_combat_and_ordinary_removal_ends_their_support() {
    for (tier, rarity) in Rarity::ALL.into_iter().enumerate() {
        let mut game = prepared(
            &[
                item(Family::SwarmBanner, rarity),
                item(Family::Grove, Rarity::Common),
            ],
            &[("Plains", 20), ("Disenchant", 10), ("Benalish Hero", 10)],
        );
        let player = game.state().turn.active;
        let opponent = PlayerId::new(1 - player.get());
        main(&mut game, 1, player);
        land(&mut game, player);
        cast(&mut game, player, "Benalish Hero", 1, &[]);
        resolve(&mut game);
        main(&mut game, 3, player);
        land(&mut game, player);
        let hero = creatures(game.state(), player, false)[0];
        assert_eq!(
            game.state().object(hero).unwrap().characteristics().power,
            Some(1)
        );
        let (power, toughness) = [(2, 1), (2, 2), (3, 2), (3, 3)][tier];
        for owner in [player, opponent] {
            for plant in creatures(game.state(), owner, true) {
                let stats = game.state().object(plant).unwrap().characteristics();
                assert_eq!(
                    (stats.power, stats.toughness),
                    (Some(power), Some(toughness))
                );
                assert_eq!(stats.keywords.contains(KeywordSet::TRAMPLE), tier == 3);
            }
        }
        until(&mut game, |game| {
            matches!(game.pending(), Pending::ChooseAttackers { .. })
        });
        let Pending::ChooseAttackers { attackers, .. } = game.pending() else {
            unreachable!()
        };
        let plant = *attackers
            .iter()
            .find(|id| game.state().object(**id).unwrap().token.is_some())
            .unwrap();
        game.apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: vec![(plant, Defender::Player(opponent))],
            },
        )
        .unwrap();
        until(&mut game, |game| {
            game.state().turn.step == Step::CombatEnd
                && matches!(game.pending(), Pending::Priority { player: asked, .. } if *asked == player)
        });
        assert_eq!(
            game.state().players[opponent.get() as usize].life,
            20 - i32::from(power)
        );
        let banner = source(&game, player, GROWTH_RELICS[1][tier]);
        cast(&mut game, player, "Disenchant", 2, &[banner]);
        resolve(&mut game);
        for plant in creatures(game.state(), player, true) {
            let stats = game.state().object(plant).unwrap().characteristics();
            assert_eq!((stats.power, stats.toughness), (Some(1), Some(1)));
            assert!(!stats.keywords.contains(KeywordSet::TRAMPLE));
        }
        for plant in creatures(game.state(), opponent, true) {
            assert_eq!(
                game.state().object(plant).unwrap().characteristics().power,
                Some(power)
            );
        }
    }
}
