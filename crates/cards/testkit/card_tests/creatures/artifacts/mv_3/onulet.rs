//! `cards/creatures/artifacts/mv_3/onulet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Onulet — {3} — 2/2 artifact creature: "When this creature dies, you
/// gain 2 life."
///
/// "Dies" is "is put into a graveyard from the battlefield" (CR 700.4),
/// and the trigger is the card's only line: the creature changes zone, the
/// life changes, and nothing else about the board does. "You" is the
/// ability's controller (CR 109.5), so the two life go to the seat that
/// controlled the Onulet and not to its opponent.
#[test]
fn onulet_gains_two_life_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[onulet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let body = on_battlefield(&engine, p0, onulet()).expect("seated");

    kill(&mut engine, body);

    assert!(
        in_graveyard(&engine, p0, onulet()).is_some(),
        "it went to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "its controller gained 2 from the dies trigger"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the other seat gained nothing"
    );
}
