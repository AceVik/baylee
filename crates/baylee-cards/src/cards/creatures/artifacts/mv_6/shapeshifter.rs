//! Shapeshifter — {6} — Artifact Creature — Shapeshifter
//! Oracle: As this creature enters, choose a number between 0 and 7.
//! Oracle: At the beginning of your upkeep, you may choose a number between 0 and 7.
//! Oracle: Shapeshifter's power is equal to the last chosen number and its toughness is equal to 7 minus that number.
//! Set: ME4 #226 — Masters Edition IV | Scryfall ID: f15f2638-3895-459a-84af-fb91de06c395 | Oracle ID: 82a6d89d-9215-4540-b7d5-26cdd6afb05b
// PARTIAL — every sentence is off the card; nothing chooses or stores a number.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHAPESHIFTER,
    oracle_id = "82a6d89d-9215-4540-b7d5-26cdd6afb05b",
    scryfall_id = "f15f2638-3895-459a-84af-fb91de06c395",
    coverage = Coverage::Partial(
        "no vocabulary chooses or stores a number, so the 0-to-7 choice and \
         the characteristic-defining power/toughness cannot be written"
    ),
    faces = &[face!(
        name = "Shapeshifter",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(0),
        toughness = Some(0),
    ),],
    // NOT SUPPORTED: "As this creature enters, choose a number between 0 and
    // 7." — no `EnterModifier` asks for a number.
    // NOT SUPPORTED: "At the beginning of your upkeep, you may choose a
    // number between 0 and 7." — no `Effect` asks for a number either, and
    // `Pending::ChooseNumber` is reached only by an announced X or a damage
    // share.
    // NOT SUPPORTED: "Shapeshifter's power is equal to the last chosen
    // number and its toughness is equal to 7 minus that number." — nothing
    // stores a chosen number and no `Modifier` reads one, so the face keeps
    // its generated 0/0 base.
    abilities = &[],
);
