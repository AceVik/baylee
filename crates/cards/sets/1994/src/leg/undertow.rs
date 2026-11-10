//! Undertow — {2}{U} — Enchantment
//! Oracle: Creatures with islandwalk can be blocked as though they didn't have islandwalk.
//! Set: LEG #82 — Legends | Scryfall ID: cf05e5c9-b7e4-4bd8-ab73-b54565710527 | Oracle ID: 1dc51112-ee5d-492a-9c79-80ebcad60bc8
// IMPLEMENTED — a layer-6 static strips islandwalk from every creature that
// has it, so it can be blocked as though it didn't have islandwalk.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNDERTOW,
    oracle_id = "1dc51112-ee5d-492a-9c79-80ebcad60bc8",
    scryfall_id = "cf05e5c9-b7e4-4bd8-ab73-b54565710527",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Undertow",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::ISLANDWALK)]),
        Modifier::RemoveKeyword(KeywordSet::ISLANDWALK)
    )],
);
