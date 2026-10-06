//! `cards/artifacts/mv_2/thought_vessel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Thought Vessel` is an artifact costing `{2}` under `Coverage::Implemented`.
/// It prints "You have no maximum hand size." and "{T}: Add {C}."
/// It taps for colorless mana, and with 9 cards in hand its static ability
/// ensures the cleanup step passes without demanding any discard.
#[test]
fn thought_vessel_taps_for_colorless_and_removes_maximum_hand_size() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let hand_cards = [
        forest(),
        forest(),
        forest(),
        forest(),
        forest(),
        forest(),
        forest(),
        forest(),
        forest(),
    ];
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thought_vessel()])
        .hand(0, &hand_cards)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vessel = on_battlefield(&engine, p0, thought_vessel())
        .expect("Thought Vessel is on the battlefield");

    activate(&mut engine, p0, thought_vessel(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "produced one colorless mana"
    );
    assert!(is_tapped(&engine, vessel), "Thought Vessel is tapped");

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        9,
        "retained all 9 cards past cleanup due to no maximum hand size"
    );
}
