//! Field of Dreams — {U} — World Enchantment
//! Oracle: Players play with the top card of their libraries revealed.
//! Set: LEG #55 — Legends | Scryfall ID: 6a63e119-3b1b-4964-a4b9-b10170ff542b | Oracle ID: 6dcc62f8-2fd9-473b-9e6a-2dd7e610f855

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIELD_OF_DREAMS,
    oracle_id = "6dcc62f8-2fd9-473b-9e6a-2dd7e610f855",
    scryfall_id = "6a63e119-3b1b-4964-a4b9-b10170ff542b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(Filter::Any, Modifier::RevealAllLibraryTops),],
    faces = &[face!(
        name = "Field of Dreams",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);
