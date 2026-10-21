//! Shyft — {4}{U} — Creature — Shapeshifter
//! Oracle: At the beginning of your upkeep, you may have this creature become the color or colors of your choice. (This effect lasts indefinitely.)
//! Set: ME2 #66 — Masters Edition II | Scryfall ID: d7f2bbf6-65b7-41c4-a219-f8ea4aeb9bbc | Oracle ID: 40c3007a-be6c-4400-9139-12b9ca098ac4
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "At the beginning of your upkeep, you may have this creature become the color or colors of your choice. (This effect lasts indefinitely.)" ColorSubject only supports Target and Attached, not Source.
card!(
    index = index::SHYFT,
    oracle_id = "40c3007a-be6c-4400-9139-12b9ca098ac4",
    scryfall_id = "d7f2bbf6-65b7-41c4-a219-f8ea4aeb9bbc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "plays as a 4/2 blue Shapeshifter; the upkeep ability to have Shyft become the color or colors of your choice is not supported because ColorSubject lacks a Source variant",
    ),
    faces = &[face!(
        name = "Shyft",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(4),
        toughness = Some(2),
    ),],
);
