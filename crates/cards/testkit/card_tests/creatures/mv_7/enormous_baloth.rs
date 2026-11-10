//! `cards/creatures/mv_7/enormous_baloth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn enormous_baloth() -> CardIndex {
    card_index("6189cd17-61aa-420a-ba53-5ddaf2bbc2ba")
}

/// Enormous Baloth — a vanilla 7/7 for {6}{G}.
#[test]
fn enormous_baloth_is_a_vanilla_seven_seven_for_seven() {
    let mut engine = table7(&lots(forest(), 7), &[enormous_baloth()], &[]);
    let baloth = cast_and_resolve(&mut engine, enormous_baloth());
    assert_body(&engine, baloth, (7, 7), KeywordSet::default());
    assert_eq!(keywords(&engine, baloth), KeywordSet::default(), "vanilla");
}

/// Blocked by a 2/2 it kills it and is unscathed apart from the 2 damage.
#[test]
fn enormous_baloth_wins_a_fight_with_an_ogre() {
    let (mut engine, baloth, ogre) = duel_vs_ogre(enormous_baloth(), &[]);
    fight(&mut engine, &[baloth], &[(ogre, baloth)]);
    assert!(in_graveyard(&engine, PlayerId::new(1), gray_ogre()).is_some());
    assert!(on_battlefield(&engine, PlayerId::new(0), enormous_baloth()).is_some());
    assert_eq!(life_of(&engine, 1), 20, "no trample, the blocker soaked it");
}
