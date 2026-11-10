//! Standardize — {U}{U} — Instant
//! Oracle: Choose a creature type other than Wall. Each creature becomes that type until end of turn.
//! Set: ONS #116 — Onslaught | Scryfall ID: f2c79e64-91bf-4e87-a4fd-3136ea67c5bb | Oracle ID: 2d4e0e0f-7167-40a4-ad76-2d5b45223eaa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STANDARDIZE,
    oracle_id = "2d4e0e0f-7167-40a4-ad76-2d5b45223eaa",
    scryfall_id = "f2c79e64-91bf-4e87-a4fd-3136ea67c5bb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Standardize",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
