//! `cards/enchantments/mv_5/berserkers_onslaught.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn berserkers_onslaught() -> CardIndex {
    card_index("85d2948c-1a79-418d-80c4-bc1012a4d313")
}

/// "Attacking creatures you control have double strike." Not before the
/// declaration; after it, and it deals its damage twice (2 + 2).
#[test]
fn berserkers_onslaught_gives_attackers_double_strike() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[berserkers_onslaught(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    assert!(!keywords(&engine, ogre).contains(KeywordSet::DOUBLE_STRIKE));
    declare_attack(&mut engine, p0, &[ogre], p1);
    assert!(keywords(&engine, ogre).contains(KeywordSet::DOUBLE_STRIKE));
    finish_combat(&mut engine);
    assert_eq!(engine.state().players[1].life, 16, "2 damage, twice");
}

/// "…creatures you control": an opposing attacker gets nothing.
#[test]
fn berserkers_onslaught_does_not_arm_the_opponents_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[berserkers_onslaught()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    declare_attack(&mut engine, p1, &[ogre], p0);
    assert!(!keywords(&engine, ogre).contains(KeywordSet::DOUBLE_STRIKE));
}
