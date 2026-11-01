//! Mistform Sliver — {1}{U} — Creature — Illusion Sliver
//! Oracle: All Slivers have "{1}: This permanent becomes the creature type of your choice in addition to its other types until end of turn."
//! Set: LGN #46 — Legions | Scryfall ID: 79a53c29-6753-4f6b-b4ee-00c1adf7e9c6 | Oracle ID: d29f59a4-b956-40e0-acd4-c2560e538baf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFORM_SLIVER,
    oracle_id = "d29f59a4-b956-40e0-acd4-c2560e538baf",
    scryfall_id = "79a53c29-6753-4f6b-b4ee-00c1adf7e9c6",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mistform Sliver",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION, subtypes::creature::SLIVER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
