//! `cards/creatures/mv_2/degavolver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn degavolver() -> CardIndex {
    card_index("9431c81f-7b71-4c5d-a63c-1b5aba03601b")
}

fn granted_ability(engine: &Engine<RegistryLookup>, object: ObjectId) -> Option<u32> {
    priority_offer(engine)
        .abilities
        .into_iter()
        .find_map(|(src, idx)| (src == object).then_some(idx))
}

/// Casts Degavolver off floating mana and answers `{1}{B}` (0) and `{R}` (1)
/// in Oracle order.
fn cast_degavolver(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    kick_black: bool,
    kick_red: bool,
) {
    cast_with_floating(engine, seat, degavolver());
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
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{1}{B}"));
                        saw_0 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_black)).unwrap();
                    }
                    1 => {
                        assert!(saw_0);
                        assert!(!saw_1);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{R}"));
                        saw_1 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_red)).unwrap();
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
fn degavolver_unkicked_is_1_1_with_no_counters_regenerate_or_first_strike() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), swamp(), swamp(), mountain()])
        .hand(0, &[degavolver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_degavolver(&mut engine, p0, false, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, degavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, volver), (1, 1));
    assert!(!keywords(&engine, volver).contains(KeywordSet::FIRST_STRIKE));
    assert!(granted_ability(&engine, volver).is_none());
}

#[test]
fn degavolver_first_kicker_enters_3_3_with_pay_3_life_regenerate() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), swamp(), swamp(), mountain()])
        .hand(0, &[degavolver()])
        .battlefield(1, &[mountain()])
        .hand(1, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_degavolver(&mut engine, p0, true, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, degavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, volver), (3, 3));
    assert!(!keywords(&engine, volver).contains(KeywordSet::FIRST_STRIKE));
    let regen_idx = granted_ability(&engine, volver).expect("regenerate ability offered");

    activate(&mut engine, p0, degavolver(), regen_idx);
    assert_eq!(engine.state().players[0].life, 17);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().object(volver).unwrap().regeneration_shields,
        1
    );

    // Opponent casts Lightning Bolt (3 lethal damage); p0 responds with Degavolver's regenerate
    // ability on p1's turn, which shields Degavolver against the Bolt.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, index::LIGHTNING_BOLT);
    aim_at(&mut engine, p1, volver);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    activate(&mut engine, p0, degavolver(), regen_idx);
    pass_until(&mut engine, |e| at_rest(e, p1));

    let saved = on_battlefield(&engine, p0, degavolver()).expect("Degavolver regenerated");
    assert!(is_tapped(&engine, saved));
    assert_eq!(engine.state().object(saved).unwrap().damage, 0);
}

#[test]
fn degavolver_second_kicker_enters_2_2_with_first_strike_and_no_regenerate() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), swamp(), swamp(), mountain()])
        .hand(0, &[degavolver()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_degavolver(&mut engine, p0, false, true);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, degavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, volver), (2, 2));
    assert!(keywords(&engine, volver).contains(KeywordSet::FIRST_STRIKE));
    assert!(granted_ability(&engine, volver).is_none());

    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    let bear = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let _ = attack_and_collect_blocks(&mut engine, volver, p1);
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, volver)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p0, degavolver()).is_some());
}

#[test]
fn degavolver_both_kickers_grant_three_counters_first_strike_and_regenerate() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), swamp(), swamp(), mountain()])
        .hand(0, &[degavolver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_degavolver(&mut engine, p0, true, true);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, degavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 3);
    assert_eq!(pt(&engine, volver), (4, 4));
    assert!(keywords(&engine, volver).contains(KeywordSet::FIRST_STRIKE));
    let regen_idx = granted_ability(&engine, volver).expect("regenerate ability offered");

    activate(&mut engine, p0, degavolver(), regen_idx);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(
        engine.state().object(volver).unwrap().regeneration_shields,
        1
    );
}
