//! `cards/creatures/mv_4/oboro_envoy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// ---------------------------------------------------------------------------
// c11: eleven cards written by the DeepSeek lane, played here by the
// coordinator. The cross rule (scripts/llm/README.md) puts the test in
// another hand than the card, and with the Gemini lane's quota spent for the
// next three hours that hand is this one.
// ---------------------------------------------------------------------------

/// Oboro Envoy: the shrink is read **after** the land it charges has landed
/// in the hand it counts.
///
/// "…gets -X/-0 until end of turn, where X is the number of cards in your
/// hand" with a cost of "return a land you control to its owner's hand" is a
/// sentence that answers itself: the returned land is in the hand by the time
/// the ability resolves, because a cost is paid on activation (CR 601.2h) and
/// the amount is read on resolution (CR 608.2f). Two cards in hand plus the
/// land is three, so the wurm is a 3/6 — a reader counting the hand at
/// announcement would leave it a 4/6.
#[test]
fn oboro_envoy_counts_the_land_it_returned_to_pay_for_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(381, forest())
        .battlefield(0, &[forest(), forest(), forest(), oboro_envoy()])
        .hand(0, &[island(), island()])
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the wurm is seated");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        2,
        "two cards in hand before the ability is paid for"
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, oboro_envoy(), 0);
    // The cost picks the land, then the ability picks its target.
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the driver answered every question the card asked"
    );

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        3,
        "the returned land is the third card in hand"
    );
    assert_eq!(
        pt(&engine, wurm),
        (3, 6),
        "three cards in hand is -3/-0 on a 6/6, and toughness is untouched"
    );
}
