//! `cards/enchantments/mv_3/lifegift.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lifegift — {2}{G} enchantment: "Whenever a land enters, you may gain 1
/// life." The whole card is that *may*, so the printed question is answered
/// both ways across two land drops: declined, the life total is exactly where
/// it was, accepted, it is one higher — a trigger that gained on its own would
/// already read 21 after the first land, and one that never fired would still
/// read 20 after the second. Three Forests pay the {2}{G} and nothing else on
/// the board moves a life total, so neither point of life can be read as
/// anything but a land arriving.
#[test]
fn lifegift_asks_for_a_life_when_a_land_enters_and_takes_the_answer_it_is_given() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[lifegift(), forest(), forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    // {2}{G} off the three Forests. The enchantment arriving is no land
    // arriving, so nothing is asked on the way in and no life moves with it.
    cast_from_hand(&mut engine, p0, lifegift());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, lifegift()).is_some(),
        "the enchantment resolved onto the battlefield"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "its own arrival asks nothing — it watches lands and it is not one: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "Lifegift entering is worth no life by itself"
    );

    // First land drop. The land entering puts the trigger on the stack, and
    // the walk stops on the question it resolves into rather than answering
    // it, because the answer is what this test is about.
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this question")
    };
    assert_eq!(
        prompt,
        YesNoPrompt::MayDo,
        "the card prints \"you may\", so the trigger is a question and not a gain"
    );
    assert_eq!(
        player, p0,
        "\"you\" is the enchantment's controller, and it is the seat asked"
    );
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declining a may is always a legal answer");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the first land arrived and the may was declined, so the life total \
         never moved — an unconditional trigger would already be at 21"
    );

    // A second land needs a turn of its own (CR 305.2a). Nobody does anything
    // on the way round, so the board p0 comes back to is the same board.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("the same question, answered the other way");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        21,
        "the second land arrived and the may was taken: one land, one life"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the enchantment's controller, not to the table"
    );
    assert!(
        on_battlefield(&engine, p0, lifegift()).is_some(),
        "the trigger is the enchantment's, so it is still standing afterwards"
    );
}
