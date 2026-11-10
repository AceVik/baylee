//! `cards/creatures/mv_5/angel_of_mercy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "a2daaf32-dbfe-4618-892e-0da24f63a44a"

/// Angel of Mercy prints two lines and each would pass a weaker test on its
/// own: a 3/3 with flying says nothing about the life and the three life says
/// nothing about the body. One cast reads both, and in the order the rules put
/// them — the Angel lands as a 3/3 flier and its enters-the-battlefield
/// trigger waits on the stack, so the controller's life is still at twenty
/// beside a creature that is already on the battlefield, and only the
/// resolution of that trigger moves it. The opponent's life is the control
/// that "you gain 3 life" is the seat that cast the Angel and not the table.
#[test]
fn angel_of_mercy_lands_as_a_flying_three_three_and_gains_its_controller_three_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[angel_of_mercy()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing has been gained before the Angel is cast"
    );

    cast_from_hand(&mut engine, p0, angel_of_mercy());
    // The spell resolves and its own enters-the-battlefield trigger is what is
    // left on the stack: the life has not moved while that waits, which is the
    // half a test that stopped at "life is 23" could never see.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, angel_of_mercy()).is_some() && !stack_is_empty(e)
    });
    let angel = on_battlefield(&engine, p0, angel_of_mercy()).expect("the Angel landed");
    assert_eq!(pt(&engine, angel), (3, 3), "the body the card prints");
    assert!(
        keywords(&engine, angel).contains(KeywordSet::FLYING),
        "and the printed flying line reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the gain is a triggered ability and not an entry modifier, so it is \
         still on the stack while the Angel stands on the battlefield"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        23,
        "one trigger, three life — and no second helping from a re-entry"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you gain 3 life\" belongs to the seat that cast the Angel, not to \
         the table"
    );
    assert!(
        on_battlefield(&engine, p0, angel_of_mercy()).is_some(),
        "and the Angel stays where it landed once the trigger has gone"
    );
}
