//! `cards/creatures/mv_7/risen_sanctuary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn risen_sanctuary() -> CardIndex {
    card_index("a2198d36-4e8c-4d64-bda5-a95d85398155")
}

/// Risen Sanctuary — an 8/8 with vigilance for {5}{G}{W}.
#[test]
fn risen_sanctuary_costs_seven_and_is_an_eight_eight() {
    let board = [lots(forest(), 6), vec![plains()]].concat();
    let mut engine = table7(&board, &[risen_sanctuary()], &[]);
    let sanctuary = cast_and_resolve(&mut engine, risen_sanctuary());
    assert_body(&engine, sanctuary, (8, 8), KeywordSet::VIGILANCE);
}

/// Vigilance: it attacks and stays untapped.
#[test]
fn risen_sanctuary_attacks_without_tapping() {
    let (mut engine, sanctuary, _ogre) = duel_vs_ogre(risen_sanctuary(), &[]);
    fight(&mut engine, &[sanctuary], &[]);
    assert_eq!(life_of(&engine, 1), 12);
    assert!(!is_tapped(&engine, sanctuary));
}
