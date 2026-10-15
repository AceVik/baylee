//! `cards/enchantments/mv_6/collective_blessing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn collective_blessing() -> CardIndex {
    card_index("7f7049e8-49ed-46da-89f7-1e40aefb3b0c")
}

/// "Creatures you control get +3/+3." Mine become 5/5, theirs stay 2/2.
#[test]
fn collective_blessing_pumps_my_creatures_by_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[collective_blessing(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    assert_eq!(pt(&engine, mine), (5, 5));
    assert_eq!(pt(&engine, theirs), (2, 2));
}
