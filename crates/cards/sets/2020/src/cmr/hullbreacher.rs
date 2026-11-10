//! Hullbreacher — {2}{U} — Creature — Merfolk Pirate
//! Oracle: Flash
//! Oracle: If an opponent would draw a card except the first one they draw in each of their draw steps, instead you create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
//! Set: CMR #74 — Commander Legends | Scryfall ID: 4df8aabc-7fcb-4b7b-980b-18f499e6c170 | Oracle ID: 2f533667-e29b-4bec-897a-e7a9eee08314
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HULLBREACHER,
    oracle_id = "2f533667-e29b-4bec-897a-e7a9eee08314",
    scryfall_id = "4df8aabc-7fcb-4b7b-980b-18f499e6c170",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hullbreacher",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::PIRATE],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[AbilityDef::Replacement(
        ReplacementRule::OpponentsExtraDrawsCreateToken(&crate::tokens::TREASURE),
    )],
);
