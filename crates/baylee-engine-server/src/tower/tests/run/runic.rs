#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::mana::ManaColor;
use v2::catalog::{RunicRecipe, Sigil, Vessel};

mod ai;
mod events;
mod feedback;

fn definition(sigil: Sigil, vessel: Vessel, rarity: Rarity) -> Definition {
    item(Family::Runic(RunicRecipe { sigil, vessel }), rarity)
}

#[test]
fn all_ten_native_rewards_pay_exact_costs_and_apply_each_rarity() {
    let player = PlayerId::new(0);
    for vessel in Vessel::ALL {
        for rarity in Rarity::ALL {
            let def = definition(Sigil::Dawn, vessel, rarity);
            let level = vessel.level(usize::from(rarity.tier() - 1)).unwrap();
            let mut game = opening(&[def], &["Memnite"]);
            cast_free(&mut game, "Memnite");
            land(&mut game);
            let per_upkeep = level.gathered + u16::from(rarity != Rarity::Common);
            let turn = u32::from(level.charges.div_ceil(per_upkeep)) * 2 - 1;
            main(&mut game, turn.max(3), player);
            land(&mut game);
            if level.mana > 0 {
                float_mana(&mut game, usize::from(level.mana));
            }
            let id = source(&game, player, def);
            let stock = charges(&game, id);
            let life = game.state().players[0].life;
            let enemy = game.state().players[1].life;
            let hand = game.state().zones.list(ZoneLocation::Hand(player)).len();
            let board = game.state().zones.list(ZoneLocation::Battlefield).len();
            let creature = permanent(&game, "Memnite");
            let stats = game
                .state()
                .arena
                .get(creature)
                .unwrap()
                .characteristics()
                .clone();
            game.apply(
                player,
                PlayerAction::ActivateAbility {
                    source: id,
                    ability_index: 1,
                },
            )
            .unwrap();
            settle_reward(&mut game, vessel, level.amount);
            assert_eq!(
                charges(&game, id),
                stock - level.charges,
                "{vessel:?} {rarity:?}"
            );
            match vessel {
                Vessel::Chalice => {
                    assert_eq!(game.state().players[0].life, life + i32::from(level.amount));
                }
                Vessel::Scepter => assert_eq!(
                    game.state().players[1].life,
                    enemy - i32::from(level.amount)
                ),
                Vessel::Seed | Vessel::Horn => assert_eq!(
                    game.state().zones.list(ZoneLocation::Battlefield).len(),
                    board + usize::from(level.amount)
                ),
                Vessel::Tome => assert_eq!(
                    game.state().zones.list(ZoneLocation::Hand(player)).len(),
                    hand + 1
                ),
                Vessel::Lens => assert_eq!(
                    game.state().zones.list(ZoneLocation::Hand(player)).len(),
                    hand
                ),
                Vessel::Bulwark => assert_eq!(
                    game.state()
                        .arena
                        .get(creature)
                        .unwrap()
                        .characteristics()
                        .toughness,
                    stats
                        .toughness
                        .map(|n| n + i16::try_from(level.amount).unwrap())
                ),
                Vessel::Banner => assert_eq!(
                    game.state()
                        .arena
                        .get(creature)
                        .unwrap()
                        .characteristics()
                        .power,
                    stats
                        .power
                        .map(|n| n + i16::try_from(level.amount).unwrap())
                ),
                Vessel::Prism | Vessel::Flask => {}
            }
        }
    }
}

#[test]
fn activation_caps_and_mythic_free_upkeep_reward_are_enforced_by_engine() {
    let player = PlayerId::new(0);
    for rarity in Rarity::ALL {
        let def = definition(Sigil::Dawn, Vessel::Chalice, rarity);
        let mut game = prepared(&[def], &[], 17);
        main(&mut game, 19, player);
        let id = source(&game, player, def);
        let allowed = match rarity {
            Rarity::Common | Rarity::Uncommon => 1,
            Rarity::Rare => 2,
            Rarity::MythicRare => 3,
        };
        let action = PlayerAction::ActivateAbility {
            source: id,
            ability_index: 1,
        };
        for _ in 0..allowed {
            game.apply(player, action.clone()).unwrap();
            resolve(&mut game);
        }
        assert!(
            charges(&game, id) >= 3,
            "remaining charges isolate the turn cap"
        );
        if rarity == Rarity::MythicRare {
            // Rare's draw-life bonus is retained at Mythic: ten ordinary draws.
            assert_eq!(game.state().players[0].life, 20 + 10 * (6 + 1) + 3 * 6);
        } else {
            assert!(game.apply(player, action.clone()).is_err());
            main(&mut game, 21, player);
            game.apply(player, action).unwrap();
            resolve(&mut game);
        }
    }
}

fn settle_reward(game: &mut Game, vessel: Vessel, amount: u16) {
    let player = PlayerId::new(0);
    if vessel == Vessel::Flask {
        assert!(matches!(game.pending(), Pending::ChooseColor { .. }));
        game.apply(player, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
        assert_eq!(
            game.state().players[0].mana_pool.available(ManaColor::Blue),
            1
        );
        assert!(game.state().zones.list(ZoneLocation::Stack).is_empty());
        return;
    }
    assert!(
        !game.state().zones.list(ZoneLocation::Stack).is_empty(),
        "ordinary paid ability keeps priority"
    );
    if vessel == Vessel::Prism {
        until(game, |game| {
            matches!(game.pending(), Pending::Arrange { .. })
        });
        let Pending::Arrange { cards, .. } = game.pending() else {
            unreachable!()
        };
        assert_eq!(cards.len(), usize::from(amount));
        advance(game);
    } else if vessel == Vessel::Lens {
        until(game, |game| {
            matches!(game.pending(), Pending::ChooseCards { min: 1, .. })
        });
        assert!(matches!(
            game.pending(),
            Pending::ChooseCards { min: 1, max: 1, .. }
        ));
        advance(game);
    }
    resolve(game);
}
