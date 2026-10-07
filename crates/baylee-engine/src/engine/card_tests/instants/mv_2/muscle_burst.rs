//! `cards/instants/mv_2/muscle_burst.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Muscle Burst: three, plus the copies of itself lying in every graveyard.
///
/// Two readings meet here and a fixed number would satisfy neither.
/// `Amount::Plus` is the constant the printed sentence says *before* the
/// count, and `Filter::Named` is what makes the count a count of this card
/// rather than of cards — the two Forests seeded into the same graveyard are
/// there to be ignored, and a reader matching `Filter::Any` would pump by
/// five and then by six.
///
/// The second cast is the half only a played card can show: the first copy
/// is in the graveyard by then, so X has grown by exactly one. It also
/// proves the spell never counts *itself* — it is still on the stack while
/// it resolves (CR 608.2m), so the first cast is +3/+3 and not +4/+4.
#[test]
fn muscle_burst_counts_three_plus_the_copies_in_every_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(371, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), rootbreaker_wurm()],
        )
        .hand(0, &[muscle_burst(), muscle_burst()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // Two cards in the graveyard that are not this one.
    seed_graveyard(&mut engine, p0, 2);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the wurm is seated");
    cast_from_hand(&mut engine, p0, muscle_burst());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, wurm),
        (9, 9),
        "no copy in any graveyard yet, so X is the bare 3: a 6/6 becomes a \
         9/9, and a reader counting the two Forests beside it would say 11/11"
    );

    cast_with_floating(&mut engine, p0, muscle_burst());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, wurm),
        (13, 13),
        "the first copy is in the graveyard now, so the second is +4/+4 on \
         top of the first +3/+3"
    );
}

/// The same card, on the half of its sentence that says **all** graveyards.
///
/// `ZoneSel::GraveyardAll` is what the card prints and `GraveyardYou` is the
/// neighbouring spelling that passes the test above without a word of
/// difference: here every copy is in the *opponent's* graveyard, so a count
/// that stopped at your own side would pump by three.
#[test]
fn muscle_burst_counts_the_copies_in_an_opponents_graveyard_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // The library filler is the card itself, which is how two copies reach a
    // graveyard nobody cast them from.
    let mut engine = Duel::new(372, muscle_burst())
        .battlefield(0, &[forest(), forest(), rootbreaker_wurm()])
        .hand(0, &[muscle_burst()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p1, 2);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the wurm is seated");
    cast_from_hand(&mut engine, p0, muscle_burst());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wurm),
        (11, 11),
        "three plus the two copies in the opponent's graveyard: +5/+5, where \
         a count of your own graveyard alone would be +3/+3"
    );
}
