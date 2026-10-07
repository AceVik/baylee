//! `cards/artifacts/mv_2/winter_orb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Winter Orb: "As long as this artifact is untapped, players can't untap
/// more than one land during their untap steps." Three tapped lands offer
/// exactly one to untap, named by the untap-step question; the other two
/// stay tapped.
#[test]
fn winter_orb_limits_untapping_to_one_land_while_it_is_itself_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[winter_orb(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    let orb = on_battlefield(&engine, p0, winter_orb()).expect("the Orb is seated");
    let lands = all_on_battlefield(&engine, p0, forest());
    assert_eq!(lands.len(), 3, "three lands are seated");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        for &l in &lands {
            state
                .object_mut(l)
                .expect("seated")
                .status
                .insert(Status::TAPPED);
        }
    }
    engine.refresh_offer();
    assert!(!is_tapped(&engine, orb), "the Orb itself starts untapped");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::Untap,
                ..
            }
        )
    });
    assert!(
        !is_tapped(&engine, orb),
        "still untapped: the limit still holds"
    );
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p0, "the active player determines untapping");
    assert_eq!(options.len(), 3, "the three tapped lands are the menu");
    for l in &lands {
        assert!(options.contains(l), "every tapped land is on offer");
    }
    assert_eq!((min, max), (1, 1), "exactly one land may untap");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lands[0]],
            },
        )
        .unwrap();
    assert!(!is_tapped(&engine, lands[0]), "the named land untapped");
    assert!(
        is_tapped(&engine, lands[1]) && is_tapped(&engine, lands[2]),
        "the Orb's limit kept the other two tapped"
    );
}

/// The Scryfall ruling: "If Winter Orb is tapped as your untap step begins,
/// your lands will all untap." Tapping the Orb before the untap step lifts
/// the limit entirely: every land untaps, with no untap-step question asked
/// at all.
#[test]
fn a_tapped_winter_orb_lets_every_land_untap_with_no_question_asked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[winter_orb(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    let orb = on_battlefield(&engine, p0, winter_orb()).expect("the Orb is seated");
    let lands = all_on_battlefield(&engine, p0, forest());
    assert_eq!(lands.len(), 3, "three lands are seated");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        for &l in &lands {
            state
                .object_mut(l)
                .expect("seated")
                .status
                .insert(Status::TAPPED);
        }
        state
            .object_mut(orb)
            .expect("seated")
            .status
            .insert(Status::TAPPED);
    }
    engine.refresh_offer();

    reach_their_main_phase(&mut engine, p1);
    for _ in 0..60 {
        if engine.state().turn.active == p0 && engine.state().turn.step == crate::turn::Step::Upkeep
        {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::Untap,
                ..
            } => panic!(
                "Winter Orb was tapped as the untap step began: no question \
                 should be asked at all"
            ),
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected while walking to p0's next upkeep: {other:?}"),
        }
    }
    assert_eq!(
        engine.state().turn.active,
        p0,
        "the walk reached p0's own next turn"
    );
    assert_eq!(
        engine.state().turn.step,
        crate::turn::Step::Upkeep,
        "and stopped at its upkeep, right after the untap step"
    );
    // Winter Orb's own untapping is never restricted — only how many
    // *lands* untap is — so the Orb comes back untapped in the very step
    // whose limit its tapped status lifted.
    assert!(
        !is_tapped(&engine, orb),
        "nothing keeps the Orb itself from untapping normally"
    );
    assert!(
        lands.iter().all(|&l| !is_tapped(&engine, l)),
        "tapped as the step began, the Orb lifted the limit: every land \
         untapped normally"
    );
}
