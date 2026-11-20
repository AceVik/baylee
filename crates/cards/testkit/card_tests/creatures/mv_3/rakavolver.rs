//! `cards/creatures/mv_3/rakavolver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn rakavolver() -> CardIndex {
    card_index("c00adb98-81dc-490f-9e0f-7f8546ef22ce")
}

/// Casts Rakavolver off floating mana and answers `{1}{W}` (0) and `{U}` (1)
/// in Oracle order.
fn cast_rakavolver(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    kick_white: bool,
    kick_blue: bool,
) {
    cast_with_floating(engine, seat, rakavolver());
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
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{1}{W}"));
                        saw_0 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_white)).unwrap();
                    }
                    1 => {
                        assert!(saw_0);
                        assert!(!saw_1);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{U}"));
                        saw_1 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_blue)).unwrap();
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
fn rakavolver_unkicked_is_2_2_with_no_flying_or_lifegain() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                island(),
            ],
        )
        .hand(0, &[rakavolver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_rakavolver(&mut engine, p0, false, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, rakavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, volver), (2, 2));
    assert!(!keywords(&engine, volver).contains(KeywordSet::FLYING));

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
    assert_eq!(engine.state().players[0].life, 20);
}

#[test]
fn rakavolver_first_kicker_triggers_lifegain_on_combat_and_noncombat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                island(),
                index::VIRIDIAN_LONGBOW,
            ],
        )
        .hand(0, &[rakavolver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_rakavolver(&mut engine, p0, true, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, rakavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, volver), (4, 4));
    assert!(!keywords(&engine, volver).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));

    // Combat damage (4): trigger gains 4 life (20 -> 24).
    let _ = attack_and_collect_blocks(&mut engine, volver, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && at_rest(e, p0)
    });
    assert_eq!(engine.state().players[1].life, 16);
    assert_eq!(engine.state().players[0].life, 24);

    // Next turn: equip Longbow and deal 1 noncombat damage -> trigger gains 1 life (24 -> 25).
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, index::VIRIDIAN_LONGBOW, 0);
    aim_at(&mut engine, p0, volver);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let ping_idx = priority_offer(&engine)
        .abilities
        .into_iter()
        .find(|(src, _)| *src == volver)
        .expect("Longbow granted ping ability")
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

    assert_eq!(engine.state().players[1].life, 15);
    assert_eq!(
        engine.state().players[0].life,
        25,
        "Rakavolver triggers on noncombat damage too"
    );
}

#[test]
fn rakavolver_second_kicker_enters_3_3_with_flying_and_no_lifegain() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                island(),
            ],
        )
        .hand(0, &[rakavolver()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_rakavolver(&mut engine, p0, false, true);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, rakavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, volver), (3, 3));
    assert!(keywords(&engine, volver).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    let blocks = attack_and_collect_blocks(&mut engine, volver, p1);
    assert!(!blocks.iter().any(|b| b.attackers.contains(&volver)));
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "no {{1}}{{W}} kicker -> no lifegain"
    );
}

#[test]
fn rakavolver_both_kickers_enter_5_5_with_flying_and_lifegain() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                island(),
            ],
        )
        .hand(0, &[rakavolver()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_rakavolver(&mut engine, p0, true, true);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, rakavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 3);
    assert_eq!(pt(&engine, volver), (5, 5));
    assert!(keywords(&engine, volver).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    let blocks = attack_and_collect_blocks(&mut engine, volver, p1);
    assert!(!blocks.iter().any(|b| b.attackers.contains(&volver)));
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(engine.state().players[1].life, 15);
    assert_eq!(engine.state().players[0].life, 25);
}
