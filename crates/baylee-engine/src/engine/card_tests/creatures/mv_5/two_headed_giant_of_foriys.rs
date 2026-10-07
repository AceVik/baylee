//! `cards/creatures/mv_5/two_headed_giant_of_foriys.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Two-Headed Giant of Foriys — "Trample" (CR 702.19b): blocked by a
/// 1-toughness Llanowar Elves, 1 of its 4 is lethal on the blocker and the
/// excess 3 go to the defending player; the additional-block sentence is
/// played in
/// `two_headed_giant_of_foriys_blocks_an_additional_creature_and_divides_its_damage`.
#[test]
fn two_headed_giant_of_foriys_tramples_excess_damage_over_its_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[two_headed_giant_of_foriys()])
        .battlefield(1, &[llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, two_headed_giant_of_foriys()).expect("seated");
    assert_eq!(pt(&engine, giant), (4, 4), "the body the card prints");
    assert!(
        keywords(&engine, giant).contains(KeywordSet::TRAMPLE),
        "\"Trample\" is the printed line"
    );
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .expect("the Giant declares");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, giant)],
            },
        )
        .expect("the Elf blocks");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        17,
        "trample sent the remaining 3 damage through to the defending player"
    );
}

/// Trample into a banding blocker. Two-Headed Giant of Foriys — trample —
/// attacks, blocked together by Timber Wolves and Gray Ogre. Timber
/// Wolves' own reminder text is the rule in play here: the defending
/// player divides the Giant's combat damage "among any creatures blocking
/// it" (CR 702.22j) — the blockers only; that division names no player, so
/// trample's own excess-to-the-player clause (CR 702.19b) is never on
/// offer here. Timber Wolves' 1 toughness plus Gray Ogre's 2 is one short
/// of the Giant's 4 power — 1 + 2 is one short of 4, so an ordinary
/// trample assignment (CR 702.19b) would put that point on p1 once both
/// blockers had lethal damage — but here it does not: the whole 4 stays
/// on the blockers, however p1 divides it.
#[test]
fn trample_into_a_banding_blocker_divides_among_the_blockers_and_none_tramples_over() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[two_headed_giant_of_foriys()])
        .battlefield(1, &[timber_wolves(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, two_headed_giant_of_foriys()).expect("seated");
    let wolves = on_battlefield(&engine, p1, timber_wolves()).expect("seated");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
    assert!(keywords(&engine, giant).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        pt(&engine, giant),
        (4, 4),
        "four power: more than the two blockers' combined toughness"
    );

    declare_band_attack(&mut engine, p0, p1, &[giant]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    declare_band_blocks(&mut engine, p1, &[(wolves, giant), (ogre, giant)]);

    let Pending::ChooseNumber {
        player,
        min,
        max,
        reason,
    } = next_combat_question(&mut engine)
    else {
        panic!("expected the division, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p1,
        "CR 702.22j: the defending player divides, not the Giant's own controller"
    );
    assert_eq!((min, max), (0, 4), "the Giant's whole power, any of it");
    assert_eq!(
        reason,
        crate::choice::NumberPrompt::CombatDamage {
            source: giant,
            recipient: wolves,
            index: 0,
            of: 2,
            left: 4,
        },
        "the first-declared blocker asked first"
    );
    engine.apply(p1, PlayerAction::ChooseNumber(0)).unwrap();

    assert_eq!(
        engine.state().object(wolves).map(|o| o.damage),
        Some(0),
        "the division answered — 0 to Timber Wolves — landed exactly: a \
         lethal-first assignment would have put 1 on it regardless"
    );
    assert!(
        on_battlefield(&engine, p1, timber_wolves()).is_some(),
        "0 damage, and 1 toughness: Timber Wolves survives"
    );
    assert!(
        on_battlefield(&engine, p1, gray_ogre()).is_none(),
        "the rest of the division — all 4 — landed on the Ogre instead, \
         which closes the case of a recorded division the Giant never dealt"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "nothing tramples past a banding division: it only ever reaches the \
         blockers, however p1 divides it"
    );
}

/// Two-Headed Giant of Foriys — "This creature can block an additional
/// creature each combat": the declare-blockers question names it in
/// `capacity` with `most: Some(2)`, blocking all three of the three
/// attackers is refused, and a vanilla creature (Gray Ogre) beside it still
/// blocks one at most. Blocking two is accepted, both are fully blocked (so
/// neither reaches the Giant's own controller), and its 4 power is then
/// divided between the two by that same controller (CR 510.1d).
#[allow(clippy::too_many_lines)] // one capacity, two refusals, one accepted block, one division
#[test]
fn two_headed_giant_of_foriys_blocks_an_additional_creature_and_divides_its_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[two_headed_giant_of_foriys(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 3, "three Elves are seated");
    let giant = on_battlefield(&engine, p1, two_headed_giant_of_foriys()).expect("seated");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");

    declare_band_attack(&mut engine, p0, p1, &elves);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { capacity, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on exactly this")
    };
    assert_eq!(
        capacity,
        vec![crate::choice::BlockCapacity {
            blocker: giant,
            most: Some(2)
        }],
        "only the Giant may block more than one, and up to two: {capacity:?}"
    );

    match engine.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(giant, elves[0]), (giant, elves[1]), (giant, elves[2])],
        },
    ) {
        Err(EngineError::IllegalAction(message)) => {
            assert_eq!(message, "creature cannot block that many attackers");
        }
        other => panic!("expected the capacity refusal, got {other:?}"),
    }
    match engine.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(ogre, elves[0]), (ogre, elves[1])],
        },
    ) {
        Err(EngineError::IllegalAction(message)) => {
            assert_eq!(message, "creature cannot block that many attackers");
        }
        other => panic!("a vanilla creature still blocks one at most, got {other:?}"),
    }

    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, elves[0]), (giant, elves[1]), (ogre, elves[2])],
            },
        )
        .expect("the Giant's extra block and the Ogre's ordinary one are both legal");
    assert_eq!(
        engine.state().combat.blocked_by(giant),
        vec![elves[0], elves[1]],
        "it blocks both"
    );
    assert!(
        engine.state().combat.is_blocked(elves[2]),
        "the Ogre took the third"
    );

    let Pending::ChooseNumber {
        player,
        min,
        reason: crate::choice::NumberPrompt::CombatDamage { source, .. },
        ..
    } = next_combat_question(&mut engine)
    else {
        panic!("expected the Giant's controller to divide its own damage")
    };
    assert_eq!(
        (player, source),
        (p1, giant),
        "the Giant's own controller divides it, not the attackers'"
    );
    engine
        .apply(player, PlayerAction::ChooseNumber(min))
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        20,
        "every attacker was blocked, and the damage step kept it so"
    );
}
