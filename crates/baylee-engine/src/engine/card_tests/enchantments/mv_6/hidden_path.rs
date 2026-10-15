//! `cards/enchantments/mv_6/hidden_path.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hidden_path() -> CardIndex {
    card_index("f1c9f5df-d36a-4e15-bae4-81190e4be477")
}

/// "Green creatures have forestwalk." A green Elf cannot be blocked by an
/// Ogre while the defender controls a Forest, and can be when they do not;
/// a red attacker is unaffected.
#[test]
fn hidden_path_gives_green_creatures_forestwalk() {
    assert!(
        !ogre_may_block(&[hidden_path()], llanowar_elves(), forest()),
        "forestwalk: the defender has a Forest"
    );
    assert!(
        ogre_may_block(&[hidden_path()], llanowar_elves(), plains()),
        "no Forest, no evasion"
    );
    assert!(
        ogre_may_block(&[hidden_path()], gray_ogre(), forest()),
        "a red creature has no forestwalk"
    );
}

/// Without the enchantment the Elf is blockable past a Forest.
#[test]
fn hidden_path_is_what_makes_the_elf_unblockable() {
    assert!(ogre_may_block(&[], llanowar_elves(), forest()));
}
