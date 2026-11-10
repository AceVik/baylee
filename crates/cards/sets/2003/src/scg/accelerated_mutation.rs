//! Accelerated Mutation — {3}{G}{G} — Instant
//! Oracle: Target creature gets +X/+X until end of turn, where X is the greatest mana value among permanents you control.
//! Set: SCG #109 — Scourge | Scryfall ID: 282f808c-0b58-4b98-aeda-f606a10d1a4b | Oracle ID: f8a9c279-0f11-4a22-8651-f9caf013ca3c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ACCELERATED_MUTATION,
    oracle_id = "f8a9c279-0f11-4a22-8651-f9caf013ca3c",
    scryfall_id = "282f808c-0b58-4b98-aeda-f606a10d1a4b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Accelerated Mutation",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
