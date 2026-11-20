//! `cards/creatures/mv_4/anavolver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn anavolver() -> CardIndex {
    card_index("03a4cbe2-cf35-4c2c-b035-666f994ffd09")
}

fn granted_ability(engine: &Engine<RegistryLookup>, object: ObjectId) -> Option<u32> {
    priority_offer(engine)
        .abilities
        .into_iter()
        .find_map(|(src, idx)| (src == object).then_some(idx))
}

/// Casts Anavolver off floating mana and answers `{1}{U}` (0) and `{B}` (1)
/// in Oracle order.
fn cast_anavolver(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    kick_blue: bool,
    kick_black: bool,
) {
    cast_with_floating(engine, seat, anavolver());
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
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{1}{U}"));
                        saw_0 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_blue)).unwrap();
                    }
                    1 => {
                        assert!(saw_0);
                        assert!(!saw_1);
                        assert_eq!(cost, baylee_core::mana::ManaCost::parse("{B}"));
                        saw_1 = true;
                        engine.apply(seat, PlayerAction::YesNo(kick_black)).unwrap();
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
fn anavolver_unkicked_is_3_3_with_no_counters_flying_or_regenerate() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                swamp(),
            ],
        )
        .hand(0, &[anavolver()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_anavolver(&mut engine, p0, false, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, anavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 0);
    assert_eq!(pt(&engine, volver), (3, 3));
    assert!(!keywords(&engine, volver).contains(KeywordSet::FLYING));
    assert!(granted_ability(&engine, volver).is_none());

    // Next combat: ground creature can block unkicked Anavolver.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    let blocks = attack_and_collect_blocks(&mut engine, volver, p1);
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == elves && b.attackers.contains(&volver)),
        "unkicked Anavolver lacks flying and is blockable by ground Elves"
    );
}

#[test]
fn anavolver_first_kicker_enters_5_5_with_flying_and_no_regenerate() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                swamp(),
            ],
        )
        .hand(0, &[anavolver()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_anavolver(&mut engine, p0, true, false);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, anavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 2);
    assert_eq!(pt(&engine, volver), (5, 5));
    assert!(keywords(&engine, volver).contains(KeywordSet::FLYING));
    assert!(granted_ability(&engine, volver).is_none());

    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));
    let blocks = attack_and_collect_blocks(&mut engine, volver, p1);
    assert!(
        !blocks.iter().any(|b| b.attackers.contains(&volver)),
        "ground Elves cannot block flying Anavolver: {blocks:?}"
    );
}

#[test]
fn anavolver_second_kicker_enters_4_4_with_pay_3_life_regenerate() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                swamp(),
            ],
        )
        .hand(0, &[anavolver()])
        .battlefield(1, &[mountain(), mountain()])
        .hand(1, &[index::LIGHTNING_BOLT, index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_anavolver(&mut engine, p0, false, true);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, anavolver()).unwrap();
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, volver), (4, 4));
    assert!(!keywords(&engine, volver).contains(KeywordSet::FLYING));
    let regen_idx = granted_ability(&engine, volver).expect("regenerate ability offered");

    activate(&mut engine, p0, anavolver(), regen_idx);
    assert_eq!(
        engine.state().players[0].life,
        17,
        "paying 3 life happens immediately on activation"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().object(volver).unwrap().regeneration_shields,
        1
    );

    // Opponent aims two Lightning Bolts (6 lethal damage on a 4/4); p0 responds with
    // Anavolver's Pay 3 life: Regenerate ability, and the shield saves Anavolver.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, index::LIGHTNING_BOLT);
    aim_at(&mut engine, p1, volver);
    pass_until(&mut engine, |e| at_rest(e, p1));
    cast_with_floating(&mut engine, p1, index::LIGHTNING_BOLT);
    aim_at(&mut engine, p1, volver);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    activate(&mut engine, p0, anavolver(), regen_idx);
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert_eq!(engine.state().players[0].life, 14);

    let saved = on_battlefield(&engine, p0, anavolver()).expect("Anavolver regenerated");
    assert!(is_tapped(&engine, saved));
    assert_eq!(engine.state().object(saved).unwrap().damage, 0);
}

#[test]
fn anavolver_both_kickers_grant_three_counters_flying_and_regenerate_and_double_with_doubling_season()
 {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                swamp(),
                index::DOUBLING_SEASON,
            ],
        )
        .hand(0, &[anavolver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_anavolver(&mut engine, p0, true, true);
    pass_until(&mut engine, |e| at_rest(e, p0));

    let volver = on_battlefield(&engine, p0, anavolver()).unwrap();
    // 2 counters + 1 counter = 3, doubled to 6 by Doubling Season (CR 614.16).
    assert_eq!(counters_on(&engine, volver, CounterKind::P1P1), 6);
    assert_eq!(pt(&engine, volver), (9, 9));
    assert!(keywords(&engine, volver).contains(KeywordSet::FLYING));
    let regen_idx = granted_ability(&engine, volver).expect("regenerate ability offered");

    activate(&mut engine, p0, anavolver(), regen_idx);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(
        engine.state().object(volver).unwrap().regeneration_shields,
        1
    );
}
