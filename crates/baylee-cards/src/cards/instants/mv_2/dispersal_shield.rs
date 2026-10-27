//! Dispersal Shield — {1}{U} — Instant
//! Oracle: Counter target spell if its mana value is less than or equal to the greatest mana value among permanents you control.
//! Set: SCG #33 — Scourge | Scryfall ID: 0c257df6-f275-40db-bfe3-a9291356cdf7 | Oracle ID: aefb1463-76fb-419e-8cda-8c5bfb88a3d2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISPERSAL_SHIELD,
    oracle_id = "aefb1463-76fb-419e-8cda-8c5bfb88a3d2",
    scryfall_id = "0c257df6-f275-40db-bfe3-a9291356cdf7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Dispersal Shield",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
