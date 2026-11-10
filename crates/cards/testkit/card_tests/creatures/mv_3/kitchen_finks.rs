//! `cards/creatures/mv_3/kitchen_finks.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kitchen Finks — {1}{G/W}{G/W} — 3/2 Ouphe: "When this creature enters,
/// you gain 2 life", and persist underneath it.
///
/// The half that is written is the enter trigger, so the scenario is the
/// only one that can read it: three Forests pay the hybrid cost, and the
/// life total is asked for *twice* — once while the spell is still on the
/// stack, where it must not have moved, and once after everything has
/// resolved, where it must have moved by exactly 2 and only for the
/// controller. The opponent's total is the counter-half of "you gain", and
/// the body on the table is what says the spell resolved rather than that
/// the trigger fired off something else.
///
/// The persist clause used to be the card's `Coverage::Partial` gap and is
/// played by the test below instead of here, so this one stays a reading of
/// the enter trigger on its own.
#[test]
fn kitchen_finks_gains_its_controller_two_life_and_leaves_the_opponent_alone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[kitchen_finks()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has been gained before anything is cast"
    );

    cast_from_hand(&mut engine, p0, kitchen_finks());
    assert!(
        !stack_is_empty(&engine),
        "the Finks is a spell, not a permanent yet"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the life belongs to the enters trigger and not to the cast"
    );

    pass_until(&mut engine, stack_is_empty);

    let finks = on_battlefield(&engine, p0, kitchen_finks()).expect("the Finks resolved");
    assert_eq!(
        pt(&engine, finks),
        (3, 2),
        "the body the card prints, so it is the Finks that entered"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "exactly 2, once, for the seat that controls it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you gain 2 life\" is not each player"
    );
}

/// Persist, and the enter trigger is what proves it was an *arrival* rather
/// than a card put back on the table.
///
/// Kitchen Finks is the one card in the pool where the two halves check each
/// other: the 2 life is gained a second time only if the return goes through
/// the same door a cast does, and the -1/-1 counter is what stops the third.
/// Seating it costs no life, which is the zero the first assertion reads —
/// so the 2 that follows is the return's and nothing else's.
#[test]
fn kitchen_finks_persists_back_smaller_and_gains_the_two_life_again() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(98, forest())
        .battlefield(0, &[kitchen_finks()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let finks = on_battlefield(&engine, p0, kitchen_finks()).expect("the Finks is seated");
    assert_eq!(pt(&engine, finks), (3, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].life,
        20,
        "a permanent the harness seats never entered, so nothing has been \
         gained and every point below belongs to the return"
    );

    kill(&mut engine, finks);

    let back = on_battlefield(&engine, p0, kitchen_finks()).expect("persist returned it");
    assert_eq!(
        pt(&engine, back),
        (2, 1),
        "a 3/2 with a -1/-1 counter on it"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "and it entered, so \"when this creature enters, you gain 2 life\" \
         fired for the second time"
    );

    kill(&mut engine, back);
    assert!(
        on_battlefield(&engine, p0, kitchen_finks()).is_none(),
        "the counter it came back with is what keeps it down"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "and nothing entered a third time"
    );
}
