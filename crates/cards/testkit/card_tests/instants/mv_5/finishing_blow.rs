//! `cards/instants/mv_5/finishing_blow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "3c8a6c58-f4af-4809-af86-a931313fa71e"
fn finishing_blow() -> CardIndex {
    card_index("3c8a6c58-f4af-4809-af86-a931313fa71e")
}

/// Finishing Blow: "Destroy target creature or planeswalker."
///
/// A planeswalker cannot be dealt to the board (a seeded one has no loyalty
/// and dies at once), so seat 0 casts Jace first. The menu then holds the
/// Planeswalker and the Elf and not the artifact, and the Planeswalker is the
/// one destroyed.
#[test]
fn finishing_blow_destroys_a_planeswalker_and_offers_creatures_too() {
    let lands = [vec![island(); 2], vec![swamp(); 7]].concat();
    let mut engine = table(
        918,
        &[jace_the_mind_sculptor(), finishing_blow()],
        &lands,
        &[],
        &[llanowar_elves(), quiet_artifact()],
    );
    let elf = on_battlefield(&engine, them(), llanowar_elves()).expect("their Elf");
    let rock = on_battlefield(&engine, them(), quiet_artifact()).expect("their artifact");
    tap_all_mana(&mut engine, me());
    cast_with_floating(&mut engine, me(), jace_the_mind_sculptor());
    settle(&mut engine);
    let jace = on_battlefield(&engine, me(), jace_the_mind_sculptor()).expect("Jace resolved");

    cast_with_floating(&mut engine, me(), finishing_blow());
    let menu = aim_at(&mut engine, me(), jace);
    assert!(menu.contains(&jace), "the planeswalker: {menu:?}");
    assert!(menu.contains(&elf), "and the creature: {menu:?}");
    assert!(!menu.contains(&rock), "not an artifact: {menu:?}");
    settle(&mut engine);

    assert!(
        in_graveyard(&engine, me(), jace_the_mind_sculptor()).is_some(),
        "destroyed"
    );
    assert!(on_battlefield(&engine, them(), llanowar_elves()).is_some());
}

/// The creature half, and the destruction: the Elf goes to the graveyard.
#[test]
fn finishing_blow_destroys_a_creature() {
    let mut engine = table(
        919,
        &[finishing_blow()],
        &[swamp(); 5],
        &[],
        &[llanowar_elves()],
    );
    let elf = on_battlefield(&engine, them(), llanowar_elves()).expect("their Elf");
    cast_at(&mut engine, finishing_blow(), elf);
    assert!(in_graveyard(&engine, them(), llanowar_elves()).is_some());
}
