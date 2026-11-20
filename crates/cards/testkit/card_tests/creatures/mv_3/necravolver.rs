//! `cards/creatures/mv_3/necravolver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn necravolver() -> CardIndex {
    card_index("639d16ed-daa4-4c8d-8550-5c9a29f4d9fe")
}

/// Casts Necravolver off floating mana and answers `{1}{G}` (0) and `{W}` (1)
/// in Oracle order.
fn cast_necravolver(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    kick_green: bool,
    kick_white: bool,
) {
    cast_with_floating(engine, seat, necravolver());
    let mut saw_0 = false;
    let mut saw_1 = false;
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::KickerOption { index, cost, .. },
                ..
            } => {
                assert_eq!(player, seat);
                match index {
                    0 => {
                        assert!(!saw_0);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{1}{G}"));
                        saw_0 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_green)).unwrap();
                    }
                    1 => {
                        assert!(saw_0);
                        assert!(!saw_1);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{W}"));
                        saw_1 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_white)).unwrap();
                    }
                    other => panic!("unexpected kicker index {other}"),
                }
            }
            _ => break,
        }
    }
    assert!(saw_0 && saw_1, "both dual kicker options were offered");
}

#[test]
fn necravolver_unkicked_is_2_2_with_no_trample_or_lifegain_trigger() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), forest(), forest(), plains()],
        )
        .hand(0, &[necravolver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_necravolver(&mut engine, p0, false, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, necravolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, volver), (2, 2));
    assert!(!keywords(&engine, volver).contains(KeywordSet::TRAMPLE));

    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    let _ = attack_and_collect_blocks(&mut engine, volver, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "no lifegain when unkicked"
    );
}

#[test]
fn necravolver_first_kicker_enters_4_4_with_trample_and_no_lifegain() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), forest(), forest(), plains()],
        )
        .hand(0, &[necravolver()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_necravolver(&mut engine, p0, true, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, necravolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, volver), (4, 4));
    assert!(keywords(&engine, volver).contains(KeywordSet::TRAMPLE));

    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, volver, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elves, volver)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert_eq!(engine.state().players[1].life, 17, "3 trample damage");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "no {{W}} kicker -> no lifegain"
    );
}

#[test]
fn necravolver_second_kicker_triggers_lifegain_on_both_combat_and_noncombat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                forest(),
                forest(),
                plains(),
                index::VIRIDIAN_LONGBOW,
            ],
        )
        .hand(0, &[necravolver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_necravolver(&mut engine, p0, false, true);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, necravolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, volver), (3, 3));
    assert!(!keywords(&engine, volver).contains(KeywordSet::TRAMPLE));

    // Walk to next turn so Necravolver can both attack and tap with Longbow.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));

    // Combat damage (3): gains 3 life via trigger (20 -> 23).
    let _ = attack_and_collect_blocks(&mut engine, volver, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && at_rest(e, p0)
    });
    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(engine.state().players[0].life, 23);

    // Noncombat damage: untap Necravolver? Wait, attacking tapped it! Wait for next turn to test Longbow.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));

    // Equip Viridian Longbow ({3}) onto Necravolver in main phase, then tap Necravolver to deal 1 noncombat damage.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, index::VIRIDIAN_LONGBOW, 0);
    aim_at(&mut engine, p0, volver);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let ping_idx = priority_offer(&engine)
        .abilities
        .into_iter()
        .find(|(src, _)| *src == volver)
        .expect("Longbow granted ping ability to Necravolver")
        .1;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: volver,
                ability_index: ping_idx,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().players[1].life,
        16,
        "1 noncombat ping damage"
    );
    assert_eq!(
        engine.state().players[0].life,
        24,
        "trigger fires on noncombat damage too (23 -> 24)"
    );
}

#[test]
fn necravolver_both_kickers_grant_three_counters_trample_and_lifegain() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), forest(), forest(), plains()],
        )
        .hand(0, &[necravolver()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_necravolver(&mut engine, p0, true, true);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, necravolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 3);
    assert_eq!(pt(&engine, volver), (5, 5));
    assert!(keywords(&engine, volver).contains(KeywordSet::TRAMPLE));

    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, volver, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elves, volver)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    // 1 to blocker + 4 trample to p1 = 5 total damage dealt -> gain 5 life!
    assert_eq!(engine.state().players[1].life, 16);
    assert_eq!(engine.state().players[0].life, 25);
}
