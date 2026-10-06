//! `cards/creatures/mv_1/timber_wolves.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Timber Wolves — the same band-and-block-spread contract as Benalish
/// Hero (CR 508.1e, 702.22c, 702.22h), played on a different board so the
/// assertions are not simply the Hero's copied over.
#[test]
fn timber_wolves_bands_with_an_ally_and_a_block_on_one_member_blocks_the_band() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for banded in [false, true] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[timber_wolves(), hurloon_minotaur()])
            .battlefield(1, &[craw_wurm()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let wolves = on_battlefield(&engine, p0, timber_wolves()).expect("seated");
        let minotaur = on_battlefield(&engine, p0, hurloon_minotaur()).expect("seated");
        let wurm = on_battlefield(&engine, p1, craw_wurm()).expect("seated");

        declare_band_attack(&mut engine, p0, p1, &[wolves, minotaur]);
        let Pending::ChooseCards {
            player,
            options,
            prompt,
            ..
        } = engine.pending().clone()
        else {
            panic!("expected the band question, got {:?}", engine.pending())
        };
        assert_eq!(player, p0, "the attacking player announces the band");
        assert_eq!(prompt, ChoicePrompt::Band { with: wolves });
        assert_eq!(options, vec![minotaur], "the only other attacker on offer");

        let band = if banded { vec![minotaur] } else { vec![] };
        answer_band_and_reach_blockers(&mut engine, p0, band);
        declare_band_blocks(&mut engine, p1, &[(wurm, minotaur)]);

        assert!(
            engine.state().combat.is_blocked(minotaur),
            "the declared block always lands on the ally"
        );
        assert_eq!(
            engine.state().combat.is_blocked(wolves),
            banded,
            "\"bands are blocked as a group\": Timber Wolves is blocked only in a band"
        );
        assert_eq!(
            engine.state().combat.blockers_of(wolves).contains(&wurm),
            banded,
            "the Wurm never declared a block against Timber Wolves: only banding put it there"
        );
    }
}

/// Timber Wolves — the reminder text's other clause: "you divide that
/// creature's combat damage ... among any of the creatures it's being
/// blocked by" (CR 702.22j). No band is needed for this half: a creature
/// with banding *blocking* alongside another creature is enough. The
/// attacker's damage is then divided by Timber Wolves' own controller —
/// the *defending* player — not by the attacker's own controller, who
/// ordinarily divides a multiply-blocked attacker's damage (CR 510.1c) and
/// would have no reason to spare a creature on the other side of the
/// table. Without a banding blocker anywhere in it, the defending player
/// is never handed that division at all — CR 702.22j names banding as the
/// only reason it would be. The engine's own fallback for this unbanded
/// double block — automatic assignment, all to the first-declared blocker
/// — is a simplification of the attacking player's own division under
/// CR 510.1c, which the engine does not yet ask for; it is not the rule,
/// and this counter-check does not assert it.
#[test]
#[allow(clippy::too_many_lines)] // the banding case and its vanilla counter-check, in full
fn timber_wolves_blocking_divides_the_attackers_damage_for_its_own_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[craw_wurm()])
            .battlefield(1, &[timber_wolves(), gray_ogre()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let wurm = on_battlefield(&engine, p0, craw_wurm()).expect("seated");
        let wolves = on_battlefield(&engine, p1, timber_wolves()).expect("seated");
        assert_eq!(pt(&engine, wurm), (6, 4), "six power to divide");

        declare_band_attack(&mut engine, p0, p1, &[wurm]);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
        declare_band_blocks(&mut engine, p1, &[(wolves, wurm), (ogre, wurm)]);

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
                recipient: wolves,
                index: 0,
                of: 2,
                left: 6,
            },
            "the first-declared blocker asked first"
        );
        engine.apply(p1, PlayerAction::ChooseNumber(0)).unwrap();

        assert_eq!(
            engine.state().object(wolves).map(|o| o.damage),
            Some(0),
            "the defending player spared its own banding creature"
        );
        assert!(
            on_battlefield(&engine, p1, gray_ogre()).is_none(),
            "and put the whole six on the ogre instead — which the Wurm's own \
             controller had no reason to do"
        );
    }

    {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[craw_wurm()])
            .battlefield(1, &[pearled_unicorn(), gray_ogre()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let wurm = on_battlefield(&engine, p0, craw_wurm()).expect("seated");

        declare_band_attack(&mut engine, p0, p1, &[wurm]);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        let unicorn = on_battlefield(&engine, p1, pearled_unicorn()).expect("seated");
        let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
        declare_band_blocks(&mut engine, p1, &[(unicorn, wurm), (ogre, wurm)]);

        let question = next_combat_question(&mut engine);
        assert!(
            !matches!(question, Pending::ChooseNumber { player, .. } if player == p1),
            "no banding anywhere in this block: CR 702.22j never hands the \
             defending player that division here, got {question:?}"
        );
    }
}
