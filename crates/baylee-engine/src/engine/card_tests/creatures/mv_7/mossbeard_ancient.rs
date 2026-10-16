//! `cards/creatures/mv_7/mossbeard_ancient.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn mossbeard_ancient() -> CardIndex {
    card_index("26afc8c1-c23f-4212-a2aa-386c71c4a7f9")
}

/// Mossbeard Ancient — a 7/7 trampler for {5}{G}{G}: "When this creature
/// enters, you gain 5 life."
#[test]
fn mossbeard_ancient_gains_five_when_it_enters() {
    let mut engine = table7(&lots(forest(), 7), &[mossbeard_ancient()], &[]);
    assert_eq!(life_of(&engine, 0), 20);
    let ancient = cast_and_resolve(&mut engine, mossbeard_ancient());
    assert_body(&engine, ancient, (7, 7), KeywordSet::TRAMPLE);
    assert_eq!(life_of(&engine, 0), 25);
}

/// Trample: 5 of the 7 go through a 2/2 blocker.
#[test]
fn mossbeard_ancient_tramples_over_a_blocker() {
    let (mut engine, ancient, ogre) = duel_vs_ogre(mossbeard_ancient(), &[]);
    fight(&mut engine, &[ancient], &[(ogre, ancient)]);
    assert_eq!(life_of(&engine, 1), 15);
}
