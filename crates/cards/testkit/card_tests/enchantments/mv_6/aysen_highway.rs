//! `cards/enchantments/mv_6/aysen_highway.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn aysen_highway() -> CardIndex {
    card_index("415bc26d-60fb-41ac-8006-7dd861671c14")
}

/// "White creatures have plainswalk." A white Knight cannot be blocked by an
/// Ogre while the defender controls a Plains, and can be when they do not;
/// a red attacker is unaffected.
#[test]
fn aysen_highway_gives_white_creatures_plainswalk() {
    assert!(
        !ogre_may_block(&[aysen_highway()], benalish_knight(), plains()),
        "plainswalk: the defender has a Plains"
    );
    assert!(
        ogre_may_block(&[aysen_highway()], benalish_knight(), forest()),
        "no Plains, no evasion"
    );
    assert!(
        ogre_may_block(&[aysen_highway()], gray_ogre(), plains()),
        "a red creature has no plainswalk"
    );
}

/// Without the enchantment the same Knight is blockable past a Plains, so
/// the evasion above is the card's.
#[test]
fn aysen_highway_is_what_makes_the_knight_unblockable() {
    assert!(ogre_may_block(&[], benalish_knight(), plains()));
}
