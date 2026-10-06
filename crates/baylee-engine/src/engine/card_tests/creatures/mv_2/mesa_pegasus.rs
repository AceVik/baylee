//! `cards/creatures/mv_2/mesa_pegasus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mesa Pegasus — banded with a grounded ally, a grounded blocker (no
/// flying or reach) ends up blocking the Pegasus too — exactly the case
/// "bands are blocked as a group" covers (CR 702.22h): a flier in a band
/// with a creature it never could have blocked alone. Unbanded, the same
/// declared block never touches the flier, which the offer itself never
/// lets a grounded creature block on its own.
#[test]
#[allow(clippy::too_many_lines)] // the offer check plus the banded/unbanded contrast, in full
fn mesa_pegasus_bands_with_a_grounded_ally_so_a_grounded_blocker_ends_up_blocking_the_flier() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for banded in [false, true] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[mesa_pegasus(), savannah_lions()])
            .battlefield(1, &[gray_ogre()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let pegasus = on_battlefield(&engine, p0, mesa_pegasus()).expect("seated");
        let lions = on_battlefield(&engine, p0, savannah_lions()).expect("seated");
        let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
        assert!(keywords(&engine, pegasus).contains(KeywordSet::FLYING));
        assert!(!keywords(&engine, ogre).contains(KeywordSet::FLYING));
        assert!(!keywords(&engine, ogre).contains(KeywordSet::REACH));

        declare_band_attack(&mut engine, p0, p1, &[pegasus, lions]);
        let band = if banded { vec![lions] } else { vec![] };
        answer_band_and_reach_blockers(&mut engine, p0, band);

        // The offer itself: a grounded ogre is only ever paired with the
        // grounded ally, band or no band — the enumeration reads flying off
        // the board, not off what the ogre eventually ends up blocking.
        let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
            panic!("expected the block offer, got {:?}", engine.pending())
        };
        let pairing = blockers
            .iter()
            .find(|b| b.blocker == ogre)
            .expect("the ogre may block something");
        assert!(pairing.attackers.contains(&lions), "the grounded ally");
        assert!(
            !pairing.attackers.contains(&pegasus),
            "flying: a grounded creature was never offered the Pegasus"
        );

        declare_band_blocks(&mut engine, p1, &[(ogre, lions)]);
        assert!(
            engine.state().combat.is_blocked(lions),
            "the declared block always lands on the ally"
        );
        assert_eq!(
            engine.state().combat.is_blocked(pegasus),
            banded,
            "the Pegasus is blocked only in a band — the ogre was never \
             offered it on its own"
        );
        assert_eq!(
            engine.state().combat.blockers_of(pegasus).contains(&ogre),
            banded,
            "banding put a creature there that flying alone would have kept out"
        );
    }
}

/// Mesa Pegasus — the CR 702.22j clause, played from the flier's side:
/// blocking alongside another creature (no band needed for this half) hands
/// the attacker's damage division to Mesa Pegasus' own controller — the
/// *defending* player — not the attacker's own controller, who ordinarily
/// divides a multiply-blocked attacker's damage (CR 510.1c) and would have
/// no reason to spare the second blocker.
#[test]
fn mesa_pegasus_blocking_divides_the_attackers_damage_for_its_own_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[craw_wurm()])
        .battlefield(1, &[mesa_pegasus(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, craw_wurm()).expect("seated");
    let pegasus = on_battlefield(&engine, p1, mesa_pegasus()).expect("seated");
    assert_eq!(pt(&engine, wurm), (6, 4), "six power to divide");

    declare_band_attack(&mut engine, p0, p1, &[wurm]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
    declare_band_blocks(&mut engine, p1, &[(pegasus, wurm), (ogre, wurm)]);

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
        "CR 702.22j: the defending player divides, not the Wurm's own controller"
    );
    assert_eq!((min, max), (0, 6), "the Wurm's whole power, any of it");
    assert_eq!(
        reason,
        crate::choice::NumberPrompt::CombatDamage {
            source: wurm,
            recipient: pegasus,
            index: 0,
            of: 2,
            left: 6,
        },
        "the first-declared blocker asked first"
    );
    engine.apply(p1, PlayerAction::ChooseNumber(0)).unwrap();

    assert_eq!(
        engine.state().object(pegasus).map(|o| o.damage),
        Some(0),
        "the defending player spared its own banding creature"
    );
    assert!(
        on_battlefield(&engine, p1, gray_ogre()).is_none(),
        "and put the whole six on the ogre instead — which the Wurm's own \
         controller had no reason to do"
    );
}

/// Mesa Pegasus — the CR 702.22k clause, played from the attacker's side:
/// banded with a grounded ally, a single grounded block declared on the
/// ally alone ends up blocking the Pegasus too (CR 702.22h, "bands are
/// blocked as a group"), and the blocker's own damage is then divided by
/// the *active* player — the Pegasus' own controller — among the whole
/// band, not by the blocker's own controller, who would ordinarily divide
/// a blocker's damage among multiple creatures it blocks (CR 510.1d). The
/// active player puts it all on the Pegasus and none on the ally: the
/// *opposite* of what the engine's own fallback would do with no division
/// recorded at all (put everything on the first-declared recipient, the
/// ally), so the answer given is the one that actually landed.
#[test]
fn mesa_pegasus_bands_attacking_so_a_single_blocker_divides_for_the_active_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mesa_pegasus(), savannah_lions()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pegasus = on_battlefield(&engine, p0, mesa_pegasus()).expect("seated");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("seated");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
    assert_eq!(pt(&engine, ogre), (2, 2), "two power to divide");

    declare_band_attack(&mut engine, p0, p1, &[pegasus, lions]);
    answer_band_and_reach_blockers(&mut engine, p0, vec![lions]);
    declare_band_blocks(&mut engine, p1, &[(ogre, lions)]);
    assert!(
        engine.state().combat.is_blocked(pegasus),
        "grounded and alone, the ogre could only ever have been declared \
         against the ally — the band is what put it on the flier too"
    );

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
        player, p0,
        "CR 702.22k: the active player divides, not the Ogre's own controller"
    );
    assert_eq!((min, max), (0, 2), "the ogre's power, any of it");
    assert_eq!(
        reason,
        crate::choice::NumberPrompt::CombatDamage {
            source: ogre,
            recipient: lions,
            index: 0,
            of: 2,
            left: 2,
        },
        "the declared block's recipient asked first, the band's after it"
    );
    engine.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();

    assert!(
        on_battlefield(&engine, p0, savannah_lions()).is_some(),
        "the active player put none of it on the ally, sparing it"
    );
    assert_eq!(
        engine.state().object(lions).map(|o| o.damage),
        Some(0),
        "not even a fraction reached the ally"
    );
    assert!(
        on_battlefield(&engine, p0, mesa_pegasus()).is_none(),
        "and put both points on the Pegasus instead — the reverse of the \
         engine's own fallback, which would have spared the flier and \
         killed the ally"
    );
}
