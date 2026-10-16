//! `cards/creatures/mv_7/oakgnarl_warrior.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn oakgnarl_warrior() -> CardIndex {
    card_index("5cafdf77-5500-4471-a848-89863eea2a35")
}

/// Oakgnarl Warrior — a 5/7 with vigilance and trample for {5}{G}{G}.
#[test]
fn oakgnarl_warrior_costs_seven_and_is_a_five_seven() {
    let mut engine = table7(&lots(forest(), 7), &[oakgnarl_warrior()], &[]);
    let warrior = cast_and_resolve(&mut engine, oakgnarl_warrior());
    assert_body(
        &engine,
        warrior,
        (5, 7),
        KeywordSet::VIGILANCE.union(KeywordSet::TRAMPLE),
    );
}

/// Trample over a 2/2 (3 through) and vigilance (untapped afterwards).
#[test]
fn oakgnarl_warrior_tramples_and_stays_untapped() {
    let (mut engine, warrior, ogre) = duel_vs_ogre(oakgnarl_warrior(), &[]);
    fight(&mut engine, &[warrior], &[(ogre, warrior)]);
    assert_eq!(life_of(&engine, 1), 17);
    assert!(!is_tapped(&engine, warrior), "vigilance");
}
