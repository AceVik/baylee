//! `cards/instants/mv_5/blessed_light.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "918b3860-e96c-45b5-b0e6-e82cad9f304d"
fn blessed_light() -> CardIndex {
    card_index("918b3860-e96c-45b5-b0e6-e82cad9f304d")
}

/// Blessed Light: "Exile target creature or enchantment."
///
/// The menu is the first half of the card: a creature and an enchantment are
/// on it, an artifact and a land are not. The enchantment is then exiled, which
/// is the second half: exile, not the graveyard.
#[test]
fn blessed_light_exiles_a_creature_or_an_enchantment_and_nothing_else() {
    let mut engine = table(
        902,
        &[blessed_light()],
        &[plains(); 5],
        &[],
        &[llanowar_elves(), their_enchantment(), quiet_artifact()],
    );
    let elf = on_battlefield(&engine, them(), llanowar_elves()).expect("their Elf");
    let aura = on_battlefield(&engine, them(), their_enchantment()).expect("their enchantment");
    let rock = on_battlefield(&engine, them(), quiet_artifact()).expect("their artifact");
    let land = on_battlefield(&engine, me(), plains()).expect("my Plains");

    cast_from_hand(&mut engine, me(), blessed_light());
    let menu = aim_at(&mut engine, me(), aura);
    assert!(menu.contains(&elf), "a creature: {menu:?}");
    assert!(menu.contains(&aura), "an enchantment: {menu:?}");
    assert!(!menu.contains(&rock), "not an artifact: {menu:?}");
    assert!(!menu.contains(&land), "not a land: {menu:?}");
    settle(&mut engine);

    assert!(
        exiled(&engine, them(), their_enchantment()),
        "the enchantment is exiled"
    );
    assert!(
        in_graveyard(&engine, them(), their_enchantment()).is_none(),
        "and not in the graveyard"
    );
    assert!(
        on_battlefield(&engine, them(), llanowar_elves()).is_some(),
        "the creature, not named, stays"
    );
}

/// The creature half of the same sentence, on its own board.
#[test]
fn blessed_light_exiles_a_creature_and_not_to_the_graveyard() {
    let mut engine = table(
        903,
        &[blessed_light()],
        &[plains(); 5],
        &[],
        &[llanowar_elves()],
    );
    let elf = on_battlefield(&engine, them(), llanowar_elves()).expect("their Elf");
    cast_at(&mut engine, blessed_light(), elf);
    assert!(exiled(&engine, them(), llanowar_elves()), "exiled");
    assert!(
        in_graveyard(&engine, them(), llanowar_elves()).is_none(),
        "and not dead"
    );
}
