//! `cards/creatures/mv_3/moriok_rigger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Moriok Rigger is a {2}{B} 2/2 whose whole text is one trigger: "Whenever an
/// artifact is put into a graveyard from the battlefield, you may put a +1/+1
/// counter on this creature." Nothing about that sentence is readable off the
/// card file — not that the question is optional, not that the counter is one,
/// and not that the trigger survives the artifact's own death — so the Rigger
/// is cast, a real artifact *creature* is destroyed by the opponent's
/// Vindicate, and the Rigger is read at 2/2 before the question and 3/3 after
/// it was answered.
#[test]
fn moriok_rigger_grows_when_an_artifact_is_put_into_a_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        // {2}{B} off three Swamps, and the artifact the Rigger is watching for
        // is an artifact *creature*: whichever way the engine reads "dies", a
        // 1/1 artifact creature reaching a graveyard satisfies it.
        .battlefield(0, &[swamp(), swamp(), swamp(), baleful_strix()])
        .hand(0, &[moriok_rigger()])
        // Vindicate is {1}{W}{B}: two Plains and a Swamp pay for it.
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, moriok_rigger());
    pass_until(&mut engine, stack_is_empty);
    let rigger = on_battlefield(&engine, p0, moriok_rigger()).expect("the Rigger resolved");
    let strix = on_battlefield(&engine, p0, baleful_strix()).expect("the Strix is on the table");
    assert_eq!(
        pt(&engine, rigger),
        (2, 2),
        "a printed 2/2 while no artifact has gone to a graveyard"
    );

    // The artifact dies to a spell from the other seat, so the trigger is
    // reading a real battlefield-to-graveyard move and not a harness shortcut.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on Vindicate's target question")
    };
    assert!(
        options.contains(&strix),
        "\"target permanent\" reaches the artifact creature: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![strix],
            },
        )
        .unwrap();

    // The Rigger's controller is the seat asked, and `MayDo` is the word that
    // says the counter is offered rather than imposed.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the Rigger's question")
    };
    assert_eq!(player, p0, "the Rigger's controller answers for the Rigger");
    assert!(
        in_graveyard(&engine, p0, baleful_strix()).is_some(),
        "the artifact is already in a graveyard when the question is asked"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, rigger),
        (3, 3),
        "exactly one +1/+1 counter for one artifact put into a graveyard \
         from the battlefield"
    );
}
