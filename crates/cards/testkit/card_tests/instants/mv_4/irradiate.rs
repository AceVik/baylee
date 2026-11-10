//! `cards/instants/mv_4/irradiate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// A pump that **counts**. The three below are one reading of the card-script
// reference — `NumAtt$ ±X` where `X` is a `Count$Valid`, which no reader took
// until `Amount::Negated` gave the sign somewhere to live — and they are
// three cards because the reading has three halves that can each be wrong on
// their own: the direction, whose permanents are counted, and whether "on the
// battlefield" means anybody's. `amount_sign_tests` carries the pool-wide
// claim the three of them cannot make.

/// Irradiate: "-1/-1 until end of turn **for each artifact you control**".
///
/// Two claims on one board. The sign — a shrink, and `resolve::counters::signed`
/// is the only thing that decides it, so reverting that call to the
/// `matches!(a, Amount::NegX | Amount::NegXFixed(_))` it replaced makes this
/// a 8/8 instead of a 4/4 and the card hands out the opposite of what it
/// prints. And the count — the opponent's artifact is on the battlefield too,
/// and `Filter::YOUR_ARTIFACT` is the difference between two and three.
#[test]
fn irradiate_shrinks_by_the_artifacts_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(311, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                chromatic_lantern(),
                chromatic_lantern(),
                rootbreaker_wurm(),
            ],
        )
        .hand(0, &[irradiate()])
        .battlefield(1, &[chromatic_lantern()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the wurm is seated");
    assert_eq!(pt(&engine, wurm), (6, 6), "the premise: a 6/6");

    cast_from_hand(&mut engine, p0, irradiate());
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
        (4, 4),
        "\"-1/-1 for each artifact you control\" with two of them on your \
         side and one on theirs: down by two, not up by two and not down by \
         three"
    );
}
