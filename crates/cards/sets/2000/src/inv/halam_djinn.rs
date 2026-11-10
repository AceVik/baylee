//! Halam Djinn — {5}{R} — Creature — Djinn
//! Oracle: Haste
//! Oracle: This creature gets -2/-2 as long as red is the most common color among all permanents or is tied for most common.
//! Set: INV #146 — Invasion | Scryfall ID: 369ade1f-e909-47ae-bb01-19588269ad8f | Oracle ID: fb85d8d1-6938-48bf-9a6d-d4616b05b4f6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HALAM_DJINN,
    oracle_id = "fb85d8d1-6938-48bf-9a6d-d4616b05b4f6",
    scryfall_id = "369ade1f-e909-47ae-bb01-19588269ad8f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Halam Djinn",
        mana_cost = mana!("{5}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(6),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
