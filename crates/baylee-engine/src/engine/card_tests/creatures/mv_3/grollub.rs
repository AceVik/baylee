//! `cards/creatures/mv_3/grollub.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn grollub() -> CardIndex {
    card_index("22c8c952-2713-418e-8259-aea09b4e740b")
}

#[test]
fn grollub_gains_opponent_life_when_dealt_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[grollub(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let beast = on_battlefield(&engine, p0, grollub()).unwrap();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    kit::settle(&mut engine, p0, |_, p| {
        kit::choose_targets(p, &[beast], &[])
    });

    assert_eq!(engine.state().players[1].life, 21);
}
