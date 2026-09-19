//! Irradiate — {3}{B} — Instant
//! Oracle: Target creature gets -1/-1 until end of turn for each artifact you control.
//! Set: MRD #67 — Mirrodin | Scryfall ID: 7e0460cf-ff87-4cf8-89b5-a8b9fb7322e0 | Oracle ID: 84d45389-a085-44bc-a3fb-1a5f7cc6cbe0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IRRADIATE,
    oracle_id = "84d45389-a085-44bc-a3fb-1a5f7cc6cbe0",
    scryfall_id = "7e0460cf-ff87-4cf8-89b5-a8b9fb7322e0",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Irradiate",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
