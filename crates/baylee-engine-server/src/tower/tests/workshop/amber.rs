#[allow(clippy::wildcard_imports)]
use super::*;

fn charges(game: &baylee_engine::state::GameState, source: ObjectId) -> u16 {
    game.arena
        .get(source)
        .unwrap()
        .counters
        .get(CounterKind::Charge)
}

#[test]
fn charges_are_real_costs_damage_targets_use_the_stack_and_only_mythic_repeats() {
    for (tier, rarity) in Rarity::ALL.into_iter().enumerate() {
        let mut game = prepared(
            &[
                item(Family::BloodAmber, rarity),
                item(Family::Hearthstone, Rarity::Common),
                item(Family::Grove, Rarity::Uncommon),
            ],
            &[("Plains", 40)],
        );
        let player = game.state().turn.active;
        let opponent = PlayerId::new(1 - player.get());
        let amber = source(&game, player, LIFE_RELICS[2][tier]);
        assert!(
            game.apply(
                player,
                PlayerAction::ActivateAbility {
                    source: amber,
                    ability_index: 1
                }
            )
            .is_err(),
            "zero charges"
        );
        main(&mut game, 3, player);
        assert_eq!(charges(game.state(), amber), 4);
        let before = game.state().players[opponent.get() as usize].life;
        game.apply(
            player,
            PlayerAction::ActivateAbility {
                source: amber,
                ability_index: 1,
            },
        )
        .unwrap();
        assert!(matches!(game.pending(), Pending::ChooseTargets { .. }));
        game.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![opponent],
            },
        )
        .unwrap();
        assert_eq!(charges(game.state(), amber), 2);
        assert_eq!(
            game.state().players[opponent.get() as usize].life,
            before,
            "damage waits for resolution"
        );
        resolve(&mut game);
        assert_eq!(
            game.state().players[opponent.get() as usize].life,
            before - [1, 2, 3, 3][tier]
        );
        until(
            &mut game,
            |game| matches!(game.pending(), Pending::Priority { player: asked, .. } if *asked == player),
        );
        let again = game.apply(
            player,
            PlayerAction::ActivateAbility {
                source: amber,
                ability_index: 1,
            },
        );
        if rarity == Rarity::MythicRare {
            again.unwrap();
            game.apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![opponent],
                },
            )
            .unwrap();
            resolve(&mut game);
            assert_eq!(
                game.state().players[opponent.get() as usize].life,
                before - 6
            );
            assert_eq!(charges(game.state(), amber), 0);
        } else {
            assert!(
                again.is_err(),
                "ordinary tiers must tap even with two charges left"
            );
        }
    }
}

#[test]
fn expert_reads_the_native_charge_ability_and_aims_at_the_opposing_side() {
    let preset = setup(
        &[
            item(Family::BloodAmber, Rarity::Rare),
            item(Family::Hearthstone, Rarity::Common),
            item(Family::Grove, Rarity::Uncommon),
        ],
        &[("Plains", 40)],
    );
    let identity = test_identity();
    let mut game = Hosted::recorded_for_execution(&preset, &identity).unwrap();
    let player = PlayerId::new(1);
    let amber = *game
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|id| {
            let object = game.state().arena.get(**id).unwrap();
            object.base_controller == player
                && object
                    .token
                    .is_some_and(|token| std::ptr::eq(token, LIFE_RELICS[2][2]))
        })
        .unwrap();
    let mut activated = false;
    let mut targeted = false;
    let mut record = Vec::new();
    for _ in 0..512 {
        if targeted && game.state().zones.list(ZoneLocation::Stack).is_empty() {
            break;
        }
        if game.needs_pump() {
            let choosing =
                activated && !targeted && matches!(game.pending(), Pending::ChooseTargets { .. });
            game.pump_slice(1);
            let accepted = game.take_record();
            for action in recorded_expert_actions(&accepted) {
                if matches!(action, PlayerAction::ActivateAbility { source, ability_index: 1 } if source == amber)
                {
                    activated = true;
                } else if choosing {
                    let (objects, players) = match action {
                        PlayerAction::ChooseTargets { objects, players } => (objects, players),
                        PlayerAction::ChooseObjects { objects } => (objects, vec![]),
                        _ => panic!("expected a damage target"),
                    };
                    assert_eq!(objects.len() + players.len(), 1);
                    assert!(players.iter().all(|seat| *seat != player));
                    assert!(
                        objects.iter().all(|id| game
                            .state()
                            .arena
                            .get(*id)
                            .unwrap()
                            .base_controller
                            != player)
                    );
                    targeted = true;
                }
            }
            record.extend(accepted);
        } else {
            let human = PlayerId::new(0);
            let action = game.house_action(human).unwrap();
            game.act_slice(human, action, 0).unwrap();
        }
    }
    assert!(
        activated && targeted,
        "Expert must actually use the charged item"
    );
    assert_eq!(
        game.state()
            .arena
            .get(amber)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        0
    );
    record.extend(game.take_record());
    let replay =
        baylee_gamehost::record::replay_exact_with_registry::<NoLoopDetection, Registry>(&record)
            .unwrap();
    assert_eq!(replay.engine.snapshot_hash(), game.snapshot_hash());
}
