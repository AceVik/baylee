//! Independent Alpha artifacts review.

#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

#[test]
fn clockwork_beast_independent_full_seven_and_zero_x_do_not_add_counters_and_foe_upkeep_is_illegal()
{
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for x in [0, 2] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[clockwork_beast(), forest(), forest(), forest(), forest()],
            )
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let beast = on_battlefield(&engine, p0, clockwork_beast()).unwrap();
        pass_until(&mut engine, |e| {
            e.state().turn.active == p1
                && e.state().turn.step == crate::turn::Step::Upkeep
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
        });
        let Pending::Priority { legal, .. } = engine.pending() else {
            unreachable!()
        };
        assert!(
            !legal.abilities.contains(&(beast, 1)),
            "another player's upkeep is not your upkeep"
        );
        pass_until(&mut engine, |e| {
            e.state().turn.active == p0
                && e.state().turn.step == crate::turn::Step::Upkeep
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
        });
        tap_all_mana(&mut engine, p0);
        activate(&mut engine, p0, clockwork_beast(), 1);
        let Pending::ChooseNumber { player, .. } = engine.pending().clone() else {
            panic!("X expected");
        };
        engine.apply(player, PlayerAction::ChooseNumber(x)).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(beast_counters(&engine, beast), 7);
        assert_eq!(engine.state().players[0].mana_pool.total(), 4 - x);
        assert!(
            engine
                .state()
                .object(beast)
                .unwrap()
                .status
                .contains(Status::TAPPED),
            "X0 still pays the tap cost"
        );
    }
}

#[test]
fn clockwork_beast_independent_external_counters_above_seven_are_not_clamped_by_recharge() {
    let p0 = PlayerId::new(0);
    let season = card_index("01546b7d-a233-4176-8843-d732074dc5b6");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                season,
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[clockwork_beast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, clockwork_beast());
    pass_until(&mut engine, stack_is_empty);
    let beast = on_battlefield(&engine, p0, clockwork_beast()).unwrap();
    assert_eq!(
        beast_counters(&engine, beast),
        14,
        "Doubling Season can exceed seven"
    );
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, clockwork_beast(), 1);
    let Pending::ChooseNumber { player, .. } = engine.pending().clone() else {
        panic!("X expected");
    };
    engine.apply(player, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        beast_counters(&engine, beast),
        14,
        "recharge does nothing above seven instead of removing excess counters"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
}

#[test]
fn clockwork_beast_independent_fog_does_not_stop_combat_counter_loss() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let fog = card_index("27e9db49-7af7-4bef-ad4c-bf5dfb92030d");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[clockwork_beast()])
        .battlefield(1, &[forest()])
        .hand(1, &[fog])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let beast = on_battlefield(&engine, p0, clockwork_beast()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(beast, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    cast_from_hand(&mut engine, p1, fog);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[1].life,
        20,
        "Fog prevented combat damage"
    );
    assert_eq!(
        beast_counters(&engine, beast),
        6,
        "combat participation is sufficient, damage is irrelevant"
    );
}

#[test]
fn clockwork_beast_independent_old_combat_trigger_does_not_remove_a_new_incarnations_counter() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[clockwork_beast(), plains()])
        .hand(0, &[ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let beast = on_battlefield(&engine, p0, clockwork_beast()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(beast, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd && !stack_is_empty(e)
    });
    assert_eq!(
        beast_counters(&engine, beast),
        7,
        "trigger is on stack and has not removed its counter"
    );
    cast_from_hand(&mut engine, p0, ephemerate());
    aim(&mut engine, vec![beast], vec![]);
    pass_until(&mut engine, stack_is_empty);
    let returned = on_battlefield(&engine, p0, clockwork_beast()).unwrap();
    assert_eq!(
        beast_counters(&engine, returned),
        7,
        "blinked creature is a fresh incarnation"
    );
}

#[test]
fn clockwork_beast_independent_upkeep_recharge_is_optional_and_capped_at_seven() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for chosen in [0, 1] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[clockwork_beast(), forest(), forest(), forest(), forest()],
            )
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let beast = on_battlefield(&engine, p0, clockwork_beast()).unwrap();
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(beast, Defender::Player(p1))],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
        });
        assert_eq!(beast_counters(&engine, beast), 6);
        let turn = engine.state().turn.number;
        pass_until(&mut engine, |e| {
            e.state().turn.number > turn
                && e.state().turn.active == p0
                && e.state().turn.step == crate::turn::Step::Upkeep
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
        });
        tap_all_mana(&mut engine, p0);
        activate(&mut engine, p0, clockwork_beast(), 1);
        let Pending::ChooseNumber {
            player, min, max, ..
        } = engine.pending().clone()
        else {
            panic!("activation asks X: {:?}", engine.pending());
        };
        assert!(min <= 4 && max >= 4);
        engine.apply(player, PlayerAction::ChooseNumber(4)).unwrap();
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });
        let Pending::ChooseNumber {
            player, min, max, ..
        } = engine.pending().clone()
        else {
            unreachable!()
        };
        assert_eq!((min, max), (0, 1), "X4 has only one free counter slot");
        let wire = serde_json::to_value(engine.pending()).unwrap();
        let roundtrip: Pending = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(roundtrip).unwrap(),
            wire,
            "numeric counter choice survives the wire"
        );
        assert!(
            engine.apply(player, PlayerAction::ChooseNumber(2)).is_err(),
            "above-cap numeric actions are refused"
        );
        assert_eq!(
            serde_json::to_value(engine.pending()).unwrap(),
            wire,
            "an illegal answer leaves the pending choice intact"
        );
        assert_eq!(beast_counters(&engine, beast), 6);
        engine
            .apply(player, PlayerAction::ChooseNumber(chosen))
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            beast_counters(&engine, beast),
            6 + u16::try_from(chosen).unwrap()
        );
        assert!(
            engine
                .state()
                .object(beast)
                .unwrap()
                .status
                .contains(Status::TAPPED)
        );
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            0,
            "full chosen X paid even when placing fewer"
        );
    }
}

#[test]
fn clockwork_beast_independent_attack_loses_one_counter_only_at_combat_end() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[clockwork_beast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let beast = on_battlefield(&engine, p0, clockwork_beast()).unwrap();
    assert_eq!(beast_counters(&engine, beast), 7);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(beast, Defender::Player(p1))],
            },
        )
        .unwrap();
    assert_eq!(
        beast_counters(&engine, beast),
        7,
        "attacking does not remove the counter immediately"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().players[1].life,
        13,
        "combat damage uses seven power"
    );
    assert_eq!(beast_counters(&engine, beast), 6);
    assert_eq!(pt(&engine, beast), (6, 4));
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.phase == Phase::SecondMain
            && stack_is_empty(e)
    });
    assert_eq!(
        beast_counters(&engine, beast),
        6,
        "history resets for a later combat it did not join"
    );
}

#[test]
fn clockwork_beast_independent_nonparticipant_keeps_seven_counters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[clockwork_beast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let beast = on_battlefield(&engine, p0, clockwork_beast()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(beast_counters(&engine, beast), 7);
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(
        legal.abilities.iter().all(|(source, _)| *source != beast),
        "upkeep-only recharge is absent in main phase"
    );
}

#[test]
fn clockwork_beast_independent_block_also_loses_one_counter() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let body = card_index("8f1dae40-b307-446e-bbd2-86aa35813871");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[body])
        .battlefield(1, &[clockwork_beast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let attacker = on_battlefield(&engine, p0, body).unwrap();
    let beast = on_battlefield(&engine, p1, clockwork_beast()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(beast, attacker)],
            },
        )
        .unwrap();
    assert_eq!(beast_counters(&engine, beast), 7);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(beast_counters(&engine, beast), 6);
    assert_eq!(engine.state().players[1].life, 20);
    assert!(in_graveyard(&engine, p0, body).is_some());
}
