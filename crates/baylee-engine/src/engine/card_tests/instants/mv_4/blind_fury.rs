//! `cards/instants/mv_4/blind_fury.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BLIND_FURY, WAR_MAMMOTH};

#[test]
fn blind_fury_removes_trample_from_all_creatures_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[BLIND_FURY])
        .battlefield(
            0,
            &[WAR_MAMMOTH, mountain(), mountain(), mountain(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mammoth = on_battlefield(&engine, p0, WAR_MAMMOTH).unwrap();
    assert!(
        engine
            .state()
            .object(mammoth)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE)
    );

    cast_from_hand(&mut engine, p0, BLIND_FURY);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .object(mammoth)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE),
        "Blind Fury should remove trample from all creatures until end of turn"
    );
}
