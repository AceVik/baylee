//! Zanam Djinn — {5}{U} — Creature — Djinn
//! Oracle: Flying
//! Oracle: This creature gets -2/-2 as long as blue is the most common color among all permanents or is tied for most common.
//! Set: INV #90 — Invasion | Scryfall ID: 57a3c1d5-0ca8-443b-ae7a-66e0363e377b | Oracle ID: 049fc248-4e46-43de-986d-fa023a3e8792
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZANAM_DJINN,
    oracle_id = "049fc248-4e46-43de-986d-fa023a3e8792",
    scryfall_id = "57a3c1d5-0ca8-443b-ae7a-66e0363e377b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Zanam Djinn",
        mana_cost = mana!("{5}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(5),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
