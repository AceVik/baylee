//! Sulam Djinn — {5}{G} — Creature — Djinn
//! Oracle: Trample
//! Oracle: This creature gets -2/-2 as long as green is the most common color among all permanents or is tied for most common.
//! Set: INV #212 — Invasion | Scryfall ID: 7aeab16f-e104-47e7-81c7-b6e0123120d7 | Oracle ID: 01e311d2-8fe6-4e64-9575-3ef266985f0b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SULAM_DJINN,
    oracle_id = "01e311d2-8fe6-4e64-9575-3ef266985f0b",
    scryfall_id = "7aeab16f-e104-47e7-81c7-b6e0123120d7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Sulam Djinn",
        mana_cost = mana!("{5}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
