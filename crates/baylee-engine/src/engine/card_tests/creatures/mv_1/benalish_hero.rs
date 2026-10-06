//! `cards/creatures/mv_1/benalish_hero.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Benalish Hero — "Banding": a band forms with an ally at the attack
/// declaration (CR 508.1e, 702.22c), and once formed, a block declared on
/// only the ally blocks the Hero too — "Bands are blocked as a group"
/// (CR 702.22h). Unbanded, the same declared block never touches the Hero.
///
/// A third attacker (no banding of its own) rides along so the "up to one"
/// cap (CR 702.22c) is a live board fact: naming both non-banding attackers
/// as the band is refused and the question stands, while naming just one
/// of them is accepted. The question's own `max` is asserted too — it
/// offers both, uncapped — so that refusal is shown to be CR 702.22c's
/// count of non-banding members, not this question's ordinary limit. A
/// second scenario is the stronger proof: a second banding creature
/// (Timber Wolves) rides in the same band as the Hero and one non-banding
/// ally, and is accepted outright, since only the ally counts against the
/// cap.
#[test]
#[allow(clippy::too_many_lines)] // the banded/unbanded loop plus the stronger-proof scenario, in full
fn benalish_hero_bands_with_an_ally_and_a_block_on_one_member_blocks_the_band() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for banded in [false, true] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[benalish_hero(), savannah_lions(), hurloon_minotaur()])
            .battlefield(1, &[gray_ogre()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let hero = on_battlefield(&engine, p0, benalish_hero()).expect("seated");
        let lions = on_battlefield(&engine, p0, savannah_lions()).expect("seated");
        let minotaur = on_battlefield(&engine, p0, hurloon_minotaur()).expect("seated");
        let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");

        declare_band_attack(&mut engine, p0, p1, &[hero, lions, minotaur]);
        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            prompt,
            ..
        } = engine.pending().clone()
        else {
            panic!("expected the band question, got {:?}", engine.pending())
        };
        assert_eq!(player, p0, "the attacking player announces the band");
        assert_eq!(prompt, ChoicePrompt::Band { with: hero });
        assert_eq!(min, 0, "attacking without a band is a legal answer");
        assert_eq!(
            options,
            vec![lions, minotaur],
            "both other attackers are on offer, neither of them banding"
        );
        assert_eq!(
            max, 2,
            "the menu itself offers both — it is not capped at one pick, so \
             the refusal below is CR 702.22c's own count of non-banding \
             members, not this question's ordinary maximum"
        );

        let refused = engine.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lions, minotaur],
            },
        );
        assert!(
            refused.is_err(),
            "CR 702.22c: \"up to one\" attacking creature without banding, \
             and this board offers two"
        );
        let Pending::ChooseCards { prompt, .. } = engine.pending().clone() else {
            panic!(
                "a refused answer leaves the band question standing, got {:?}",
                engine.pending()
            )
        };
        assert_eq!(
            prompt,
            ChoicePrompt::Band { with: hero },
            "the same question — same leader — is still open after the refusal"
        );

        let band = if banded { vec![lions] } else { vec![] };
        answer_band_and_reach_blockers(&mut engine, p0, band);
        assert_eq!(
            engine.state().combat.band_of(hero).is_some(),
            banded,
            "the Hero joined a band only when one was named"
        );

        declare_band_blocks(&mut engine, p1, &[(ogre, lions)]);
        assert!(
            engine.state().combat.is_blocked(lions),
            "the declared block always lands on the ally"
        );
        assert_eq!(
            engine.state().combat.is_blocked(hero),
            banded,
            "\"bands are blocked as a group\": the Hero is blocked only in a band"
        );
        assert_eq!(
            engine.state().combat.blockers_of(hero).contains(&ogre),
            banded,
            "the ogre never declared a block against the Hero: only banding put it there"
        );
    }

    // The stronger proof: the cap counts non-banding members, not band
    // size. Timber Wolves also has banding, so a band of the Hero, Timber
    // Wolves and one non-banding ally (Savannah Lions) holds only one
    // creature without banding and is accepted outright.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[benalish_hero(), timber_wolves(), savannah_lions()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hero = on_battlefield(&engine, p0, benalish_hero()).expect("seated");
    let wolves = on_battlefield(&engine, p0, timber_wolves()).expect("seated");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("seated");

    declare_band_attack(&mut engine, p0, p1, &[hero, wolves, lions]);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the band question, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![wolves, lions],
        "both other attackers on offer, one of them also banding"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wolves, lions],
            },
        )
        .expect(
            "CR 702.22c: \"one or more attacking creatures with banding and \
             up to one attacking creature without banding\" — Timber Wolves' \
             own banding does not count against the cap, and Lions is the \
             only one that does",
        );
    let mates = engine.state().combat.band_mates(hero);
    assert!(
        mates.contains(&wolves) && mates.contains(&lions),
        "the band formed with both, not just one of them: {mates:?}"
    );
}

/// Benalish Hero's reminder text: "you divide that creature's combat
/// damage, not its controller, among any of the creatures it's being
/// blocked by or is blocking" (CR 702.22k). Banded with an ally, a single
/// blocker ends up blocking both (CR 702.22h), and its damage is divided by
/// the *active* player — the Hero's own controller — not by the blocker's
/// own controller, who would ordinarily be the one dividing a blocker's
/// damage (CR 510.1d). The active player puts it all on the Hero and none
/// on the ally: the *opposite* of what the engine's own fallback would do
/// with no division recorded at all (put everything on the first-declared
/// recipient, the ally), so the answer given is the one that actually
/// landed. Unbanded, the same block touches only the ally and nothing is
/// ever divided.
#[test]
#[allow(clippy::too_many_lines)] // one loop, both branches of the division read out in full
fn benalish_heros_band_divides_a_blockers_damage_by_the_active_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for banded in [false, true] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[benalish_hero(), savannah_lions()])
            .battlefield(1, &[gray_ogre()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let hero = on_battlefield(&engine, p0, benalish_hero()).expect("seated");
        let lions = on_battlefield(&engine, p0, savannah_lions()).expect("seated");
        let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
        assert_eq!(pt(&engine, ogre), (2, 2), "two power to divide");

        declare_band_attack(&mut engine, p0, p1, &[hero, lions]);
        let band = if banded { vec![lions] } else { vec![] };
        answer_band_and_reach_blockers(&mut engine, p0, band);
        declare_band_blocks(&mut engine, p1, &[(ogre, lions)]);

        let question = next_combat_question(&mut engine);
        if banded {
            let Pending::ChooseNumber {
                player,
                min,
                max,
                reason,
            } = question
            else {
                panic!("expected the division, got {question:?}")
            };
            assert_eq!(
                player, p0,
                "CR 702.22k: the active player divides, not the blocker's own controller"
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
                on_battlefield(&engine, p0, benalish_hero()).is_none(),
                "and put both points on the Hero instead — the reverse of the \
                 engine's own fallback, which would have spared the Hero and \
                 killed the ally"
            );
        } else {
            assert!(
                !matches!(question, Pending::ChooseNumber { .. }),
                "unbanded, the ogre blocks only the ally: no division is asked, got {question:?}"
            );
            assert!(
                on_battlefield(&engine, p0, savannah_lions()).is_none(),
                "the whole of the ogre's power landed on the only creature it blocks"
            );
            assert!(
                on_battlefield(&engine, p0, benalish_hero()).is_some(),
                "unblocked, the Hero was never in reach of it"
            );
        }
    }
}
