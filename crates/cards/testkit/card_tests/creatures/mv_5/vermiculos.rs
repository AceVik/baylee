//! `cards/creatures/mv_5/vermiculos.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "88cadf97-4b24-481a-9136-96c017fd96e9"

/// Vermiculos — {4}{B} 1/1 Horror: "Whenever an artifact enters, this creature
/// gets +4/+4 until end of turn."
///
/// Neither half of that sentence is visible in the card file, so the board
/// plays it twice with a bystander in between. Sol Ring is the artifact that
/// pumps, Exploration is a permanent entering on the same board that must not,
/// and the second Sol Ring is what tells two stacked +4/+4s from one — a single
/// grant, a once-per-turn trigger or a filter without the word `ARTIFACT` all
/// read differently on a 1/1. The turn afterwards is the printed duration: the
/// Horror is still standing, and it is a printed 1/1 again.
#[test]
fn vermiculos_grows_by_four_for_each_artifact_that_enters_and_only_until_the_turn_ends() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[vermiculos(), forest(), forest(), forest(), forest()])
        .hand(0, &[quiet_artifact(), exploration(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let horror = on_battlefield(&engine, p0, vermiculos()).expect("the Horror is on the table");
    assert_eq!(
        pt(&engine, horror),
        (1, 1),
        "a printed 1/1 while nothing has entered"
    );

    // (1) An artifact enters. The trigger goes on the stack behind it, so the
    // reading is taken once the board is quiet again.
    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        pt(&engine, horror),
        (5, 5),
        "\"whenever an artifact enters, this creature gets +4/+4\""
    );

    // (2) The control: a permanent that is not an artifact enters on the same
    // board and the Horror must not move. A trigger that had lost the word
    // `ARTIFACT` would read 9/9 here, one +4/+4 per permanent.
    cast_from_hand(&mut engine, p0, exploration());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, exploration()).is_some(),
        "the enchantment resolved onto the same board"
    );
    assert_eq!(
        pt(&engine, horror),
        (5, 5),
        "an enchantment is no artifact: the pump is a filter and not \
         \"whenever a permanent enters\""
    );

    // (3) A second artifact: two separate +4/+4s on a 1/1, which neither a
    // once-per-turn trigger nor a set-instead-of-add could produce.
    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        pt(&engine, horror),
        (9, 9),
        "a printed 1/1 with two +4/+4s on it"
    );

    // (4) "until end of turn": a turn later the Horror is still standing and
    // is a printed 1/1 again, so the size was a duration and not a body.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, horror),
        (1, 1),
        "the pump lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, vermiculos()).is_some(),
        "and the Horror is still there, so the size left rather than the creature"
    );
}
