//! `cards/sorceries/mv_6/extinguish_all_hope.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "c7ca25f3-7a22-477b-8546-4c2597d5d1ff"
fn extinguish_all_hope() -> CardIndex {
    card_index("c7ca25f3-7a22-477b-8546-4c2597d5d1ff")
}

// oracle_id = "5a20113c-7ea8-4edf-af9f-148ccd326a88"
fn nyx_fleece_ram() -> CardIndex {
    card_index("5a20113c-7ea8-4edf-af9f-148ccd326a88")
}

/// Extinguish All Hope: "Destroy all nonenchantment creatures."
///
/// The Nyx-Fleece Ram is a creature and an enchantment and stays; an ordinary
/// creature of each seat and an artifact creature go.
#[test]
fn extinguish_all_hope_spares_enchantment_creatures() {
    let mut engine = table(
        957,
        &[extinguish_all_hope()],
        &[swamp(); 6],
        &[nyx_fleece_ram(), llanowar_elves()],
        &[gray_ogre(), baleful_strix(), their_enchantment()],
    );
    cast_plain(&mut engine, extinguish_all_hope());
    assert!(
        on_battlefield(&engine, me(), nyx_fleece_ram()).is_some(),
        "an enchantment creature survives"
    );
    assert!(
        in_graveyard(&engine, me(), llanowar_elves()).is_some(),
        "a plain creature dies"
    );
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some());
    assert!(
        in_graveyard(&engine, them(), baleful_strix()).is_some(),
        "so does an artifact creature"
    );
    assert!(
        on_battlefield(&engine, them(), their_enchantment()).is_some(),
        "an enchantment that is no creature was never in it"
    );
}
