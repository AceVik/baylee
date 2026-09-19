//! Strength of Cedars — {4}{G} — Instant — Arcane
//! Oracle: Target creature gets +X/+X until end of turn, where X is the number of lands you control.
//! Set: CHK #245 — Champions of Kamigawa | Scryfall ID: 85ab46f5-d3e1-403f-92e7-f40de51a3d4d | Oracle ID: 61b29c75-00d0-4ddb-9e27-cfd47302830e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STRENGTH_OF_CEDARS,
    oracle_id = "61b29c75-00d0-4ddb-9e27-cfd47302830e",
    scryfall_id = "85ab46f5-d3e1-403f-92e7-f40de51a3d4d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Strength of Cedars",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::INSTANT,
        subtypes = &[subtypes::spell::ARCANE],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
