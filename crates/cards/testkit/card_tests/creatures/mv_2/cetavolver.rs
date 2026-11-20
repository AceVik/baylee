//! `cards/creatures/mv_2/cetavolver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn cetavolver() -> CardIndex {
    card_index("3ae86510-f828-48f8-adab-759df4b5544e")
}

/// Casts Cetavolver off floating mana and answers `{1}{R}` (0) and `{G}` (1)
/// in Oracle order.
fn cast_cetavolver(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    kick_red: bool,
    kick_green: bool,
) {
    cast_with_floating(engine, seat, cetavolver());
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
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{1}{R}"));
                        saw_0 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_red)).unwrap();
                    }
                    1 => {
                        assert!(saw_0);
                        assert!(!saw_1);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{G}"));
                        saw_1 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_green)).unwrap();
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
fn cetavolver_unkicked_is_1_1_with_no_counters_or_keywords() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), mountain(), mountain(), forest()])
        .hand(0, &[cetavolver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_cetavolver(&mut engine, p0, false, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, cetavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, volver), (1, 1));
    let kw = keywords(&engine, volver);
    assert!(!kw.contains(KeywordSet::FIRST_STRIKE));
    assert!(!kw.contains(KeywordSet::TRAMPLE));
}

#[test]
fn cetavolver_first_kicker_enters_3_3_with_first_strike_and_wins_combat_against_3_3() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), mountain(), mountain(), forest()])
        .hand(0, &[cetavolver()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_cetavolver(&mut engine, p0, true, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, cetavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, volver), (3, 3));
    let kw = keywords(&engine, volver);
    assert!(kw.contains(KeywordSet::FIRST_STRIKE));
    assert!(!kw.contains(KeywordSet::TRAMPLE));

    // Next turn: attack into 3/3 Hill Giant — first strike kills Hill Giant before it deals damage.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, volver, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, volver)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, p0, cetavolver()).is_some());
}

#[test]
fn cetavolver_second_kicker_enters_2_2_with_trample() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), mountain(), mountain(), forest()])
        .hand(0, &[cetavolver()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_cetavolver(&mut engine, p0, false, true);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, cetavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, volver), (2, 2));
    let kw = keywords(&engine, volver);
    assert!(!kw.contains(KeywordSet::FIRST_STRIKE));
    assert!(kw.contains(KeywordSet::TRAMPLE));

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
    assert_eq!(
        engine.state().players[1].life,
        19,
        "1 trample damage over 1/1"
    );
}

#[test]
fn cetavolver_both_kickers_enter_4_4_with_first_strike_and_trample() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), mountain(), mountain(), forest()])
        .hand(0, &[cetavolver()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_cetavolver(&mut engine, p0, true, true);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, cetavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 3);
    assert_eq!(pt(&engine, volver), (4, 4));
    let kw = keywords(&engine, volver);
    assert!(kw.contains(KeywordSet::FIRST_STRIKE));
    assert!(kw.contains(KeywordSet::TRAMPLE));

    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, volver, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, volver)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, p0, cetavolver()).is_some());
    assert_eq!(
        engine.state().players[1].life,
        19,
        "1 trample damage over 3/3"
    );
}
