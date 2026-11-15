//! Played and hosted acceptance for the voluntary renewable Plant combination.
#[allow(clippy::wildcard_imports)]
use super::*;

const COMBINATION: [Definition; 3] = [
    Definition {
        family: Family::SeedCradle,
        rarity: Rarity::Rare,
    },
    Definition {
        family: Family::Hearthstone,
        rarity: Rarity::Common,
    },
    Definition {
        family: Family::RootReservoir,
        rarity: Rarity::MythicRare,
    },
];

#[test]
fn a_three_slot_combination_can_repeat_with_real_mana_and_the_player_can_stop() {
    // A human can continue beyond Expert's 32-token strategic target.
    const CYCLES: u16 = 40;
    let mut game = prepared(&COMBINATION, &[("Forest", 30), ("Llanowar Elves", 10)]);
    let player = game.state().turn.active;
    for turn in [1, 3] {
        main(&mut game, turn, player);
        land(&mut game, player);
    }
    let roots = source(&game, player, GROWTH_RELICS[3][3]);
    cast(&mut game, player, "Llanowar Elves", 1, &[]);
    until(&mut game, |game| {
        matches!(game.pending(), Pending::YesNo { .. })
    });
    assert_eq!(
        charges(&game, roots),
        0,
        "a creature card does not supply token charges"
    );
    for cycle in 0..CYCLES {
        game.apply(player, PlayerAction::YesNo(true)).unwrap();
        if cycle == 0 {
            mana(&mut game, player, 1);
        } else {
            game.apply(
                player,
                PlayerAction::ActivateAbility {
                    source: roots,
                    ability_index: 1,
                },
            )
            .unwrap();
        }
        game.apply(player, PlayerAction::PassPriority).unwrap();
        until(&mut game, |game| {
            matches!(game.pending(), Pending::YesNo { .. })
        });
        assert_eq!(
            creatures(game.state(), player, true).len(),
            usize::from(cycle + 1)
        );
        assert_eq!(
            charges(&game, roots),
            1,
            "one charge is earned and the previous one was really spent"
        );
        assert_eq!(
            game.state().players[player.get() as usize]
                .mana_pool
                .total(),
            0
        );
    }
    game.apply(player, PlayerAction::YesNo(false)).unwrap();
    resolve(&mut game);
    assert_eq!(
        game.state().players[player.get() as usize].life,
        21 + i32::from(CYCLES)
    );
    assert_eq!(
        creatures(game.state(), player, true).len(),
        usize::from(CYCLES)
    );
    assert!(
        matches!(game.pending(), Pending::Priority { .. }),
        "declining resumes the ordinary duel, never a detector draw"
    );
}

#[test]
fn hosted_expert_uses_renewable_mana_then_finishes_its_build_turn() {
    let mut preset = setup(&COMBINATION, &[("Forest", 30), ("Llanowar Elves", 10)]);
    let resources = [
        v2::loadout::resources_for(&[], [0, 0], 0, 3).unwrap(),
        v2::loadout::resources_for(&COMBINATION, [0, 0], 1, 3).unwrap(),
    ];
    preset.house_rules = v2::loadout::rules_for(&resources, 17, 3).unwrap();
    let identity = test_identity();
    let mut game = Hosted::recorded_for_execution(&preset, &identity).unwrap();
    let human = PlayerId::new(0);
    let expert = PlayerId::new(1);
    let mut built_at = None;
    for _ in 0..1024 {
        if creatures(game.state(), expert, true).len() >= 8 {
            built_at.get_or_insert(game.state().turn.number);
        }
        if built_at.is_some_and(|turn| game.state().turn.number > turn) {
            break;
        }
        if matches!(game.pending(), Pending::GameOver(_)) {
            break;
        }
        if game.needs_pump() {
            game.pump_slice(1);
        } else {
            game.act_slice(human, game.house_action(human).unwrap(), 0)
                .unwrap();
        }
    }
    assert!(
        built_at.is_some(),
        "Expert makes use of the renewable paid token chain"
    );
    assert!(
        game.state().turn.number > built_at.unwrap(),
        "Expert must decline optional continuation and finish its turn"
    );
    let record = game.take_record();
    let replay =
        baylee_gamehost::record::replay_exact_with_registry::<NoLoopDetection, Registry>(&record)
            .unwrap();
    assert_eq!(replay.engine.snapshot_hash(), game.snapshot_hash());
}

#[test]
fn hosted_expert_uses_an_optional_cradle_payment_and_resumes_exactly() {
    let preset = setup(
        &[
            item(Family::SeedCradle, Rarity::Rare),
            item(Family::SpringChalice, Rarity::Common),
            item(Family::RootReservoir, Rarity::MythicRare),
        ],
        &[("Forest", 40)],
    );
    let identity = test_identity();
    let mut game = Hosted::recorded_for_execution(&preset, &identity).unwrap();
    let human = PlayerId::new(0);
    for _ in 0..256 {
        if !creatures(game.state(), PlayerId::new(1), true).is_empty()
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
    assert!(
        !creatures(game.state(), PlayerId::new(1), true).is_empty(),
        "Expert actually pays to grow a Plant"
    );
    let record = game.take_record();
    assert!(recorded_expert_actions(&record).any(|action| action == PlayerAction::YesNo(true)));
    let replay =
        baylee_gamehost::record::replay_exact_with_registry::<NoLoopDetection, Registry>(&record)
            .unwrap();
    assert_eq!(replay.engine.snapshot_hash(), game.snapshot_hash());
}
