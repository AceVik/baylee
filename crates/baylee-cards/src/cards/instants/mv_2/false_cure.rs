//! False Cure — {B}{B} — Instant
//! Oracle: Until end of turn, whenever a player gains life, that player loses 2 life for each 1 life they gained.
//! Set: ONS #146 — Onslaught | Scryfall ID: ef397db1-2d99-4cb0-a6e9-6f72d615ebad | Oracle ID: fc4c969d-408c-46d1-8dba-fabecc2c30ce
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FALSE_CURE,
    oracle_id = "fc4c969d-408c-46d1-8dba-fabecc2c30ce",
    scryfall_id = "ef397db1-2d99-4cb0-a6e9-6f72d615ebad",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "False Cure",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
