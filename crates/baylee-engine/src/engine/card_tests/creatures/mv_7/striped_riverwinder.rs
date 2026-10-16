//! `cards/creatures/mv_7/striped_riverwinder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn striped_riverwinder() -> CardIndex {
    card_index("725972de-4816-44cf-a2a5-4326c3bc1b4b")
}

/// Striped Riverwinder — a 5/5 with hexproof for {6}{U}.
#[test]
fn striped_riverwinder_costs_seven_and_is_a_five_five() {
    let mut engine = table7(&lots(island(), 7), &[striped_riverwinder()], &[]);
    let serpent = cast_and_resolve(&mut engine, striped_riverwinder());
    assert_body(&engine, serpent, (5, 5), KeywordSet::HEXPROOF);
}

/// Hexproof: their Bolt may aim at the Ogre and not at the Serpent.
#[test]
fn striped_riverwinder_hexproof_keeps_their_bolt_off_it() {
    let (menu, serpent, ogre) = bolt_menu(striped_riverwinder(), 1);
    assert!(menu.contains(&ogre));
    assert!(!menu.contains(&serpent));
}

/// Cycling {U}: one Island discards it and draws a card.
#[test]
fn striped_riverwinder_cycles_for_blue() {
    cycle_it(striped_riverwinder(), &[island()], 0);
}
