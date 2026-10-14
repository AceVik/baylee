//! `cards/enchantments/mv_5/intimidation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn intimidation() -> CardIndex {
    card_index("a345d7d1-5d30-49a0-b139-a46f9419c50f")
}

/// "Creatures you control have fear": only artifact and/or black creatures
/// may block them. A red Ogre may not, a black Goblin may; and the
/// opponent's creatures are not given fear.
#[test]
fn intimidation_gives_my_creatures_fear() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[intimidation(), gray_ogre()])
        .battlefield(1, &[gray_ogre(), festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let attacker = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let red_blocker = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    let black_blocker = on_battlefield(&engine, p1, festering_goblin()).expect("their Goblin");
    assert!(keywords(&engine, attacker).contains(KeywordSet::FEAR));
    assert!(!keywords(&engine, red_blocker).contains(KeywordSet::FEAR));

    declare_attack(&mut engine, p0, &[attacker], p1);
    let offer = blocks_offered(&mut engine);
    assert!(
        offer.is_some(),
        "the black Goblin can block, so blocks are asked"
    );
    assert!(
        may_block(&offer, black_blocker, attacker),
        "a black creature may block a creature with fear"
    );
    assert!(
        !may_block(&offer, red_blocker, attacker),
        "a red, non-artifact creature may not"
    );
}
