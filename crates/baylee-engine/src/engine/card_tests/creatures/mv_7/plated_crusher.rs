//! `cards/creatures/mv_7/plated_crusher.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn plated_crusher() -> CardIndex {
    card_index("411f3ea6-36ad-4bbc-8cde-e79662a08ee9")
}

/// Plated Crusher — a 7/6 with trample and hexproof for {4}{G}{G}{G}.
#[test]
fn plated_crusher_costs_seven_and_is_a_seven_six() {
    let mut engine = table7(&lots(forest(), 7), &[plated_crusher()], &[]);
    let crusher = cast_and_resolve(&mut engine, plated_crusher());
    assert_body(
        &engine,
        crusher,
        (7, 6),
        KeywordSet::TRAMPLE.union(KeywordSet::HEXPROOF),
    );
}

/// Trample: 5 of the 7 go through a 2/2 blocker.
#[test]
fn plated_crusher_tramples_over_a_blocker() {
    let (mut engine, crusher, ogre) = duel_vs_ogre(plated_crusher(), &[]);
    fight(&mut engine, &[crusher], &[(ogre, crusher)]);
    assert_eq!(life_of(&engine, 1), 15);
}

/// Hexproof: their Bolt may aim at the Ogre and not at the Crusher.
#[test]
fn plated_crusher_hexproof_keeps_their_bolt_off_it() {
    let (menu, crusher, ogre) = bolt_menu(plated_crusher(), 1);
    assert!(menu.contains(&ogre));
    assert!(!menu.contains(&crusher));
}
