//! Shade's Breath — {1}{B} — Instant
//! Oracle: Until end of turn, each creature you control becomes a black Shade and gains "{B}: This creature gets +1/+1 until end of turn."
//! Set: ONS #167 — Onslaught | Scryfall ID: a37be9a8-ef69-4c62-8455-e129e62fe69a | Oracle ID: 6b4a4de7-ec26-45ab-a460-e6c005000bf0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHADE_S_BREATH,
    oracle_id = "6b4a4de7-ec26-45ab-a460-e6c005000bf0",
    scryfall_id = "a37be9a8-ef69-4c62-8455-e129e62fe69a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Shade's Breath",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
