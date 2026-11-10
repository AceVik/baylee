//! `cards/instants/mv_1/wirewood_pride.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wirewood Pride: the same reading with no sign in front of it.
///
/// The positive side is the larger half of what this transcoder change
/// reaches — 148 reference scripts against 26 — and it shares every line of
/// the reader with the two above except the wrapper. A test only of the
/// negatives would pass with `Amount::Negated` applied unconditionally.
#[test]
fn wirewood_pride_pumps_by_the_elves_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(313, forest())
        .battlefield(0, &[forest(), llanowar_elves(), rootbreaker_wurm()])
        .hand(0, &[wirewood_pride()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the wurm is seated");
    cast_from_hand(&mut engine, p0, wirewood_pride());
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
        (8, 8),
        "two Elves on the battlefield, one each side: +2/+2 upwards"
    );
}
