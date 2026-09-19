//! Hunger of the Nim — {1}{B} — Sorcery
//! Oracle: Target creature gets +1/+0 until end of turn for each artifact you control.
//! Set: DST #46 — Darksteel | Scryfall ID: bba50e47-2417-447f-b334-50ef0faecfae | Oracle ID: 1af3c6ff-2884-4d8c-a01f-6f74d8ea10cc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HUNGER_OF_THE_NIM,
    oracle_id = "1af3c6ff-2884-4d8c-a01f-6f74d8ea10cc",
    scryfall_id = "bba50e47-2417-447f-b334-50ef0faecfae",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Hunger of the Nim",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
