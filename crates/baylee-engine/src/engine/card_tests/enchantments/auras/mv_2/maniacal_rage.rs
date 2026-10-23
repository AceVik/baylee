//! `cards/enchantments/auras/mv_2/maniacal_rage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn maniacal_rage() -> CardIndex {
    card_index("b5dbb212-0399-475b-a831-921f7c76cdc4")
}

#[test]
fn maniacal_rage_gives_plus_two_plus_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[gray_ogre(), mountain(), mountain()])
        .hand(0, &[maniacal_rage()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bears = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, maniacal_rage());
    kit::settle(&mut engine, p0, |_, p| {
        kit::choose_targets(p, &[bears], &[])
    });

    assert_eq!(pt(&engine, bears), (4, 4));
}
