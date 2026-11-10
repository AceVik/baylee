//! `cards/creatures/mv_2/fledgling_djinn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Fledgling Djinn` prints `KeywordSet::FLYING` and an upkeep trigger dealing 1 damage to its controller
/// under `Coverage::Implemented`.
/// Advancing past the opponent's turn to the active player's next main phase processes the upkeep trigger,
/// reducing the controller's life total from 20 to 19 while leaving the opponent at 20.
#[test]
fn fledgling_djinn_deals_damage_on_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1620, forest())
        .battlefield(0, &[fledgling_djinn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let djinn = on_battlefield(&engine, p0, fledgling_djinn()).expect("djinn is seated");
    assert!(keywords(&engine, djinn).contains(KeywordSet::FLYING));

    // A *delta* and not an absolute: the Djinn was on the battlefield before
    // turn one, so its trigger has already fired once by the time this seat
    // first holds priority, and reading 20 here would be reading a board this
    // harness never sets up.
    let before = engine.state().players[0].life;
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the opponent has taken nothing"
    );

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().players[0].life,
        before,
        "\"at the beginning of *your* upkeep\" — the opponent's turn is not one"
    );

    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        before - 1,
        "and its controller's own upkeep takes exactly one more point"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"deals 1 damage to you\" never reaches across the table"
    );
}
