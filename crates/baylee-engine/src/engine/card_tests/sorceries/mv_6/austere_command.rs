//! `cards/sorceries/mv_6/austere_command.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "09cc8709-fe10-472a-b05c-e89f3523018d"
fn austere_command() -> CardIndex {
    card_index("09cc8709-fe10-472a-b05c-e89f3523018d")
}

/// The one board both choices are played on: an artifact, an enchantment, a
/// mana-value-three creature and a mana-value-five creature, all across the
/// table.
fn austere_table(seed: u64) -> Engine<RegistryLookup> {
    table(
        seed,
        &[austere_command()],
        &[plains(); 6],
        &[],
        &[quiet_artifact(), their_enchantment(), gray_ogre(), earth_elemental()],
    )
}

/// Austere Command: "Choose two — Destroy all artifacts; Destroy all
/// enchantments; Destroy all creatures with mana value 3 or less; Destroy all
/// creatures with mana value 4 or greater."
///
/// Artifacts and the big creatures: the Sol Ring and the five-mana Elemental
/// die, and the enchantment and the three-mana Ogre are left standing.
#[test]
fn austere_command_artifacts_and_big_creatures() {
    let mut engine = austere_table(943);
    cast_from_hand(&mut engine, me(), austere_command());
    let offered = choose_modes(&mut engine, me(), 0b1001);
    assert_eq!(offered.len(), 6, "choose two of four: six pairs");
    settle(&mut engine);

    assert!(in_graveyard(&engine, them(), quiet_artifact()).is_some(), "mode 1");
    assert!(in_graveyard(&engine, them(), earth_elemental()).is_some(), "mode 4");
    assert!(on_battlefield(&engine, them(), their_enchantment()).is_some(), "not chosen");
    assert!(on_battlefield(&engine, them(), gray_ogre()).is_some(), "mana value 3 is not 4 or greater");
}

/// Enchantments and the small creatures: the other pair.
#[test]
fn austere_command_enchantments_and_small_creatures() {
    let mut engine = austere_table(944);
    cast_from_hand(&mut engine, me(), austere_command());
    choose_modes(&mut engine, me(), 0b0110);
    settle(&mut engine);

    assert!(in_graveyard(&engine, them(), their_enchantment()).is_some(), "mode 2");
    assert!(in_graveyard(&engine, them(), gray_ogre()).is_some(), "mode 3");
    assert!(on_battlefield(&engine, them(), quiet_artifact()).is_some(), "not chosen");
    assert!(
        on_battlefield(&engine, them(), earth_elemental()).is_some(),
        "mana value 5 is not 3 or less"
    );
}
