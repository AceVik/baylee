//! `cards/enchantments/mv_5/gravitational_shift.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gravitational_shift() -> CardIndex {
    card_index("0411aead-9f77-435d-a343-635f23221cf7")
}

/// "Creatures with flying get +2/+0. Creatures without flying get -2/-0."
/// Both statics reach every creature at the table, whoever controls it.
#[test]
fn gravitational_shift_trades_power_for_flight() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gravitational_shift(), serra_angel(), gray_ogre()])
        .battlefield(1, &[gray_ogre(), ornithopter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let angel = on_battlefield(&engine, p0, serra_angel()).expect("Serra Angel");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let their_ogre = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    let thopter = on_battlefield(&engine, p1, ornithopter()).expect("their Ornithopter");
    assert_eq!(pt(&engine, angel), (6, 4), "a 4/4 flyer gets +2/+0");
    assert_eq!(pt(&engine, ogre), (0, 2), "a 2/2 without flying gets -2/-0");
    assert_eq!(pt(&engine, their_ogre), (0, 2), "the opponent's too");
    assert_eq!(pt(&engine, thopter), (2, 2), "a 0/2 flyer gets +2/+0");
}
