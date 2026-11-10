//! `cards/creatures/mv_5/rushwood_elemental.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Rushwood Elemental` is a 4/4 creature costing `{G}{G}{G}{G}{G}` under `Coverage::Implemented`.
/// It prints trample and "At the beginning of your upkeep, you may put a +1/+1 counter on this creature."
/// When its controller's upkeep arrives, the trigger asks `Pending::YesNo` with `YesNoPrompt::MayDo`.
/// Answering yes puts a +1/+1 counter on `Rushwood Elemental`, growing it to 5/5.
#[test]
fn rushwood_elemental_grows_with_a_counter_on_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rushwood_elemental()])
        .start();
    keep_mulligans(&mut engine);

    // On the battlefield from the start, so the first upkeep that is its
    // controller's is the one that asks — before any main phase.
    let elemental = on_battlefield(&engine, p0, rushwood_elemental())
        .expect("Rushwood Elemental is on the battlefield");
    assert_eq!(pt(&engine, elemental), (4, 4), "base body is 4/4");
    assert!(
        keywords(&engine, elemental).contains(KeywordSet::TRAMPLE),
        "Rushwood Elemental has trample"
    );
    assert_eq!(
        counters_on(&engine, elemental, CounterKind::P1P1),
        0,
        "starts with zero counters"
    );

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    assert_eq!(
        (engine.state().turn.active, engine.state().turn.step),
        (p0, Step::Upkeep),
        "asked in its controller's upkeep"
    );

    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!("expected YesNo prompt for Rushwood Elemental upkeep trigger");
    };
    assert_eq!(player, p0, "controller is asked the may-do question");
    assert_eq!(prompt, YesNoPrompt::MayDo);

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elemental, CounterKind::P1P1),
        1,
        "upkeep trigger placed one +1/+1 counter"
    );
    assert_eq!(pt(&engine, elemental), (5, 5), "body grew to 5/5");
}
