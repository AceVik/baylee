//! `cards/creatures/mv_4/reclusive_wight.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Reclusive Wight prints one sentence: "At the beginning of your upkeep, if
/// you control another nonland permanent, sacrifice this creature." Two
/// upkeeps read both halves of that condition off one board. The four Swamps
/// beneath it are lands, so a Wight standing alone is a single nonland
/// permanent and the trigger has to leave it where it is — a filter that had
/// lost `NONLAND` would have counted the Swamps, reached two and eaten the
/// Wight a turn early. Casting a Sol Ring makes the count genuinely two, and
/// the next upkeep of the Wight's own controller takes it.
#[test]
fn reclusive_wight_survives_its_upkeep_alone_and_is_sacrificed_once_another_nonland_stands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[reclusive_wight(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    // {3}{B} out of the four Swamps, which leaves the board exactly as it was:
    // the Wight is the only nonland permanent either seat controls.
    cast_from_hand(&mut engine, p0, reclusive_wight());
    pass_until(&mut engine, stack_is_empty);
    let wight = on_battlefield(&engine, p0, reclusive_wight()).expect("the Wight resolved");
    assert_eq!(pt(&engine, wight), (4, 4), "the body the card prints");

    // A whole turn round the table and back, so the walk crosses the Wight's
    // controller's upkeep — where the question this card is, is asked.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, reclusive_wight()).is_some(),
        "four Swamps are lands, so \"another nonland permanent\" is false and \
         the upkeep leaves the Wight standing"
    );
    assert!(
        in_graveyard(&engine, p0, reclusive_wight()).is_none(),
        "and nothing sacrificed it on the way past"
    );

    // One artifact is all it takes: the same board with a Sol Ring on it makes
    // the count two.
    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "a Sol Ring resolved beside the Wight"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, reclusive_wight()).is_none(),
        "\"if you control another nonland permanent, sacrifice this creature\""
    );
    assert!(
        in_graveyard(&engine, p0, reclusive_wight()).is_some(),
        "a sacrificed creature goes to its owner's graveyard, which is the \
         difference between a trigger that fired and one that quietly did nothing"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the permanent it died beside never moved"
    );
}
