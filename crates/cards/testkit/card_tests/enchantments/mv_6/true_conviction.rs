//! `cards/enchantments/mv_6/true_conviction.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn true_conviction() -> CardIndex {
    card_index("fc299c1c-50f3-492a-b6b8-a3664bb72ab7")
}

/// "Creatures you control have double strike and lifelink." The unblocked
/// Ogre deals 2 twice and I gain the 4.
#[test]
fn true_conviction_gives_double_strike_and_lifelink() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[true_conviction(), gray_ogre()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    for kw in [KeywordSet::DOUBLE_STRIKE, KeywordSet::LIFELINK] {
        assert!(keywords(&engine, ogre).contains(kw));
        assert!(!keywords(&engine, theirs).contains(kw), "only mine");
    }
    declare_attack(&mut engine, p0, &[ogre], p1);
    finish_combat(&mut engine);
    assert_eq!(engine.state().players[1].life, 16, "2 damage, twice");
    assert_eq!(engine.state().players[0].life, 24, "lifelink on both");
}
