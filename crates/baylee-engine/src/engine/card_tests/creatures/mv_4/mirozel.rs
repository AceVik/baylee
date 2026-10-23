//! `cards/creatures/mv_4/mirozel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn mirozel() -> CardIndex {
    card_index("c35111ce-4325-4547-b954-08275a268060")
}

#[test]
fn mirozel_returns_to_hand_when_targeted() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[mirozel(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let illusion = on_battlefield(&engine, p0, mirozel()).unwrap();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    kit::settle(&mut engine, p0, |_, p| {
        kit::choose_targets(p, &[illusion], &[])
    });

    assert!(on_battlefield(&engine, p0, mirozel()).is_none());
    assert!(in_hand(&engine, p0, mirozel()).is_some());
}
