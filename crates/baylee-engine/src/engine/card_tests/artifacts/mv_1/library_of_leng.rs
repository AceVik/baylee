//! `cards/artifacts/mv_1/library_of_leng.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Library of Leng — "You have no maximum hand size." (Its discard
/// replacement is not written.) p0 holds sixteen cards through its own
/// cleanup step and keeps every one.
#[test]
fn library_of_leng_leaves_no_maximum_hand_size() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let library = card_index("867def48-4be8-4056-bcf1-d6b00450b9a3");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[library])
        .hand(0, &[forest(); 9])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hand = |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Hand(p0)).len();
    let held = hand(&engine);
    assert!(held > 7, "more than seven in hand: {held}");
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(hand(&engine), held, "nothing discarded at cleanup");
}
