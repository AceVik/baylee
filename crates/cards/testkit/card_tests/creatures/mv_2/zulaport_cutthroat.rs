//! `cards/creatures/mv_2/zulaport_cutthroat.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Zulaport Cutthroat — {1}{B}, a 1/1: "Whenever this creature or another
/// creature you control dies, each opponent loses 1 life and you gain 1 life."
///
/// A three-seat table is what reads the word **each**: one death has to take a
/// life off both other seats at once, which a duel could not tell apart from
/// "target opponent". Both halves of the trigger's filter are then played with
/// Ashnod's Altar — first the Llanowar Elves beside it (another creature you
/// control) and then the Cutthroat itself — while the counter-proof comes
/// first of all: a creature dying *across* the table is just as dead, and only
/// the "you control" in that filter declines it.
#[test]
#[allow(clippy::too_many_lines)] // two deaths that count, one that must not, on a three-seat table
fn zulaport_cutthroat_drains_each_opponent_for_a_creature_its_controller_loses() {
    // The third seat is read by index below (`players[2]`) rather than by
    // handle, so only the two that name a seat to the engine are bound.
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::table(97, forest(), 3)
        .battlefield(
            0,
            &[
                forest(),
                swamp(),
                swamp(),
                swamp(),
                plains(),
                plains(),
                ashnods_altar(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[zulaport_cutthroat(), vindicate()])
        .life(0, 20)
        .life(1, 20)
        .life(2, 20)
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
            && at_rest(e, p0)
    });

    // Both spells come out of the one main phase the lands pay for: a pool
    // empties when a step ends (CR 500.5), and this test never leaves it.
    cast_from_hand(&mut engine, p0, zulaport_cutthroat());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let cutthroat =
        on_battlefield(&engine, p0, zulaport_cutthroat()).expect("the Cutthroat resolved");
    assert_eq!(pt(&engine, cutthroat), (1, 1), "the printed 1/1");

    // The counter-proof, taken while the Cutthroat is alive and watching: the
    // trigger is not "a creature dies", it is "a creature *you control* dies".
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    cast_from_hand(&mut engine, p0, vindicate());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Vindicate targets a permanent, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&theirs),
        "the Elf across the table is a permanent and a legal target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "their creature is dead, which is what makes the control say anything"
    );
    assert_eq!(
        [
            engine.state().players[0].life,
            engine.state().players[1].life,
            engine.state().players[2].life,
        ],
        [20, 20, 20],
        "a death across the table is not a death this Cutthroat watches"
    );

    // "another creature you control dies" — the Altar's price *is* the
    // creature, so the death is a real one and not a board the harness built.
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves stand beside it");
    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the sacrifice is a cost, got {:?}", engine.pending())
    };
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, asked before it is paid"
    );
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        options.len(),
        2,
        "the two creatures this seat controls: {options:?}"
    );
    assert!(options.contains(&fodder) && options.contains(&cutthroat));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(engine.state().players[0].life, 21, "\"you gain 1 life\"");
    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"each opponent loses 1 life\""
    );
    assert_eq!(
        engine.state().players[2].life,
        19,
        "and \"each\" is both of them: one trigger, two seats"
    );

    // "whenever *this* creature … dies" — the half `Filter::This` carries and
    // a filter written as `Filter::Another` alone would have dropped.
    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the second sacrifice is a cost too, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert_eq!(
        options,
        vec![cutthroat],
        "the Cutthroat is the only creature this seat still has: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cutthroat],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(engine.state().players[0].life, 22, "you gain 1 life again");
    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[2].life, 18);
    assert!(
        in_graveyard(&engine, p0, zulaport_cutthroat()).is_some(),
        "the trigger outlives the permanent that printed it"
    );
}
