//! `cards/enchantments/mv_6/rage_reflection.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn rage_reflection() -> CardIndex {
    card_index("03fb26b1-6d9c-455b-9b2f-a052a9222db7")
}

/// "Creatures you control have double strike." Mine has it and deals 2
/// twice; the opponent's does not.
#[test]
fn rage_reflection_gives_my_creatures_double_strike() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rage_reflection(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    assert!(keywords(&engine, ogre).contains(KeywordSet::DOUBLE_STRIKE));
    assert!(!keywords(&engine, theirs).contains(KeywordSet::DOUBLE_STRIKE));
    declare_attack(&mut engine, p0, &[ogre], p1);
    finish_combat(&mut engine);
    assert_eq!(engine.state().players[1].life, 16);
}
