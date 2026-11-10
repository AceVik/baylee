//! Ixidor's Will — {2}{U} — Instant
//! Oracle: Counter target spell unless its controller pays {2} for each Wizard on the battlefield.
//! Set: ONS #90 — Onslaught | Scryfall ID: 1b713448-853a-41ee-a302-963e9c1c1c65 | Oracle ID: a84c9507-d054-4770-84f3-5a4b5a01d826
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IXIDOR_S_WILL,
    oracle_id = "a84c9507-d054-4770-84f3-5a4b5a01d826",
    scryfall_id = "1b713448-853a-41ee-a302-963e9c1c1c65",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Ixidor's Will",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
