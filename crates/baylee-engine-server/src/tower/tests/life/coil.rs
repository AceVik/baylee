#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn every_tier_pays_its_mana_life_and_tap_cost_before_drawing() {
    for (tier, rarity) in Rarity::ALL.into_iter().enumerate() {
        let mut game = prepared(&[item(Family::LifeCoil, rarity)], &[("Plains", 40)]);
        let player = game.state().turn.active;
        let coil = source(&game, player, LIFE_RELICS[3][tier]);
        for turn in [1, 3, 5] {
            main(&mut game, turn, player);
            land(&mut game, player);
        }
        assert!(
            game.apply(
                player,
                PlayerAction::ActivateAbility {
                    source: coil,
                    ability_index: 0
                }
            )
            .is_err(),
            "no floating mana"
        );
        mana(&mut game, player, [3, 2, 1, 1][tier]);
        let hand = game.state().zones.list(ZoneLocation::Hand(player)).len();
        game.apply(
            player,
            PlayerAction::ActivateAbility {
                source: coil,
                ability_index: 0,
            },
        )
        .unwrap();
        assert_eq!(game.state().players[player.get() as usize].life, 18);
        assert_eq!(
            game.state().players[player.get() as usize]
                .mana_pool
                .total(),
            0
        );
        assert_eq!(
            game.state().zones.list(ZoneLocation::Hand(player)).len(),
            hand
        );
        resolve(&mut game);
        assert_eq!(
            game.state().zones.list(ZoneLocation::Hand(player)).len(),
            hand + 1
        );
        assert_eq!(
            game.state()
                .arena
                .get(coil)
                .unwrap()
                .status
                .contains(baylee_engine::object::Status::TAPPED),
            rarity != Rarity::MythicRare
        );
        if rarity == Rarity::MythicRare {
            mana(&mut game, player, 1);
            game.apply(
                player,
                PlayerAction::ActivateAbility {
                    source: coil,
                    ability_index: 0,
                },
            )
            .unwrap();
            resolve(&mut game);
            assert_eq!(game.state().players[player.get() as usize].life, 16);
            assert_eq!(
                game.state().zones.list(ZoneLocation::Hand(player)).len(),
                hand + 2
            );
        }
    }
}

#[test]
fn exact_life_payment_remains_legal_and_expert_avoids_the_lethal_purchase() {
    let mut game = prepared(
        &[item(Family::LifeCoil, Rarity::MythicRare)],
        &[("Plains", 60)],
    );
    let player = game.state().turn.active;
    let coil = source(&game, player, LIFE_RELICS[3][3]);
    let profile = v2::generation::CURRENT.expert();
    for activation in 0..10 {
        main(&mut game, 1 + activation * 2, player);
        land(&mut game, player);
        if activation == 0 {
            assert!(matches!(
                ai(&game, player, AIProfile::TOWER_EXPERT),
                PlayerAction::PassPriority
            ));
        }
        if activation == 9 {
            assert!(matches!(
                ai(&game, player, profile),
                PlayerAction::PassPriority
            ));
        }
        mana(&mut game, player, 1);
        if activation == 0 {
            let action = ai(&game, player, profile);
            assert!(
                matches!(action, PlayerAction::ActivateAbility { source, ability_index: 0 } if source == coil),
                "{action:?}"
            );
        }
        if activation == 9 {
            assert_eq!(game.state().players[player.get() as usize].life, 2);
            assert!(
                !matches!(ai(&game, player, profile), PlayerAction::ActivateAbility { source, .. } if source == coil)
            );
        }
        game.apply(
            player,
            PlayerAction::ActivateAbility {
                source: coil,
                ability_index: 0,
            },
        )
        .unwrap();
        if activation < 9 {
            resolve(&mut game);
        }
    }
    assert!(
        matches!(game.pending(), Pending::GameOver { .. }),
        "paying the last two life loses before the draw resolves"
    );
}

#[test]
fn hosted_expert_funds_life_coil_from_an_unspent_land_and_resumes() {
    use baylee_gamehost::session::tower::{ReplayBudget, resume_native};
    use std::time::{Duration, Instant};

    let preset = setup(&[item(Family::LifeCoil, Rarity::Rare)], &[("Plains", 40)]);
    let identity = test_identity();
    let mut game = Hosted::recorded_for_execution(&preset, &identity).unwrap();
    let human = PlayerId::new(0);
    for _ in 0..192 {
        if game.state().players[1].life == 18
            && game.state().zones.list(ZoneLocation::Stack).is_empty()
        {
            break;
        }
        if game.needs_pump() {
            game.pump_slice(1);
        } else {
            game.act_slice(human, game.house_action(human).unwrap(), 0)
                .unwrap();
        }
    }
    assert_eq!(
        game.state().players[1].life,
        18,
        "Expert actually pays for a draw"
    );
    let record = game.take_record();
    let actions: Vec<_> = recorded_expert_actions(&record).collect();
    assert!(actions.windows(2).any(|actions| matches!(
        actions,
        [
            PlayerAction::ActivateManaAbility { .. },
            PlayerAction::ActivateAbility {
                ability_index: 0,
                ..
            }
        ]
    )));
    let resumed = resume_native::<Registry>(
        &preset,
        &identity,
        record.as_slice(),
        ReplayBudget::default(),
        Instant::now() + Duration::from_secs(2),
    )
    .unwrap()
    .into_session(
        "life-coil-review".into(),
        vec!["Human".into(), "Expert".into()],
    );
    assert_eq!(resumed.snapshot_hash(), game.snapshot_hash());
}
