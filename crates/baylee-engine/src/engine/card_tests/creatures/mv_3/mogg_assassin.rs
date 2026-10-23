//! `cards/creatures/mv_3/mogg_assassin.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn mogg_assassin() -> CardIndex {
    card_index("36a6925a-621e-47d6-8cb3-1b2886e1986e")
}

#[test]
fn mogg_assassin_taps_and_flips_coin_against_target_opponent_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[mogg_assassin()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let assassin = on_battlefield(&engine, p0, mogg_assassin()).unwrap();
    let bears = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    kit::activate_object(&mut engine, p0, assassin, 0);
    kit::settle(&mut engine, p0, |_, p| {
        kit::choose_targets(p, &[bears], &[])
    });

    assert!(is_tapped(&engine, assassin));
}
