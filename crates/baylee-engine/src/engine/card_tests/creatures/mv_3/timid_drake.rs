//! `cards/creatures/mv_3/timid_drake.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
fn timid_drake() -> CardIndex {
    card_index("aa8c5467-190a-4c6a-872c-56b8f8d01a2b")
}

/// `Timid Drake` ({2}{U}, 3/3 Drake) has flying and returns to its owner's hand
/// when another creature enters the battlefield.
#[test]
fn timid_drake_returns_to_hand_when_another_creature_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[timid_drake(), forest()])
        .hand(0, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let drake = on_battlefield(&engine, p0, timid_drake()).expect("Timid Drake");
    assert_eq!(pt(&engine, drake), (3, 3));

    cast_from_hand(&mut engine, p0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, timid_drake()).is_none());
    assert!(in_hand(&engine, p0, timid_drake()).is_some());
}
