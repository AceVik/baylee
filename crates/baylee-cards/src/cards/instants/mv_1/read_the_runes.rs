//! Read the Runes — {X}{U} — Instant
//! Oracle: Draw X cards. For each card drawn this way, discard a card unless you sacrifice a permanent.
//! Set: JMP #167 — Jumpstart | Scryfall ID: 2c65088f-bbbd-4e8d-b482-58181069bef2 | Oracle ID: 74316be1-041f-4a2e-a296-0b496f90ca25
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::READ_THE_RUNES,
    oracle_id = "74316be1-041f-4a2e-a296-0b496f90ca25",
    scryfall_id = "2c65088f-bbbd-4e8d-b482-58181069bef2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Read the Runes",
        mana_cost = mana!("{X}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
