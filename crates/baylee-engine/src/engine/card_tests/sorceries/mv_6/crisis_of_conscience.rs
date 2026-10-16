//! `cards/sorceries/mv_6/crisis_of_conscience.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "d5862292-9ef9-4bf2-b468-45bfe0d70f7a"
fn crisis_of_conscience() -> CardIndex {
    card_index("d5862292-9ef9-4bf2-b468-45bfe0d70f7a")
}

// oracle_id = "61e2d557-f08d-489b-94c8-aff87e5189f9"
fn take_up_arms() -> CardIndex {
    card_index("61e2d557-f08d-489b-94c8-aff87e5189f9")
}

/// A table with three Warrior tokens of seat 0 already made (by Take Up Arms,
/// cast off the same mana), the Crisis still in hand and mana left floating.
fn crisis_table(seed: u64) -> Engine<RegistryLookup> {
    let mut engine = table(
        seed,
        &[take_up_arms(), crisis_of_conscience()],
        &[plains(); 11],
        &[llanowar_elves(), quiet_artifact()],
        &[gray_ogre()],
    );
    tap_all_mana(&mut engine, me());
    cast_with_floating(&mut engine, me(), take_up_arms());
    settle(&mut engine);
    assert_eq!(
        my_tokens(&engine).len(),
        3,
        "three Warriors to destroy or keep"
    );
    engine
}

/// Crisis of Conscience, first mode: "Destroy all tokens."
#[test]
fn crisis_of_conscience_destroys_all_tokens() {
    let mut engine = crisis_table(950);
    cast_in_mode(&mut engine, crisis_of_conscience(), 0);
    settle(&mut engine);

    assert!(my_tokens(&engine).is_empty(), "no token is left");
    assert!(
        on_battlefield(&engine, me(), llanowar_elves()).is_some(),
        "a card stays"
    );
    assert!(
        on_battlefield(&engine, me(), quiet_artifact()).is_some(),
        "so does an artifact"
    );
    assert!(
        on_battlefield(&engine, them(), gray_ogre()).is_some(),
        "and theirs"
    );
}

/// Second mode: "Destroy all nonland, nontoken permanents."
#[test]
fn crisis_of_conscience_destroys_nonland_nontoken_permanents() {
    let mut engine = crisis_table(951);
    cast_in_mode(&mut engine, crisis_of_conscience(), 1);
    settle(&mut engine);

    assert!(
        in_graveyard(&engine, me(), llanowar_elves()).is_some(),
        "a creature"
    );
    assert!(
        in_graveyard(&engine, me(), quiet_artifact()).is_some(),
        "an artifact"
    );
    assert!(
        in_graveyard(&engine, them(), gray_ogre()).is_some(),
        "theirs too"
    );
    assert_eq!(my_tokens(&engine).len(), 3, "the tokens are spared");
    assert!(
        on_battlefield(&engine, me(), plains()).is_some(),
        "and the lands"
    );
}
