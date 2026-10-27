//! Patriarch's Bidding — {3}{B}{B} — Sorcery
//! Oracle: Each player chooses a creature type. Each player returns all creature cards of a type chosen this way from their graveyard to the battlefield.
//! Set: MH2 #275 — Modern Horizons 2 | Scryfall ID: 317686d8-b762-4598-b74a-8b1fa6b899ba | Oracle ID: 25fc3bc2-d852-4f52-9adb-c7e35c06f3af
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PATRIARCH_S_BIDDING,
    oracle_id = "25fc3bc2-d852-4f52-9adb-c7e35c06f3af",
    scryfall_id = "317686d8-b762-4598-b74a-8b1fa6b899ba",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Patriarch's Bidding",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
