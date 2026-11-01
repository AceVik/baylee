//! Donate — {2}{U} — Sorcery
//! Oracle: Target player gains control of target permanent you control.
//! Set: UDS #31 — Urza's Destiny | Scryfall ID: 7f6d8ce9-f8c8-45ad-b74c-97fba0e2982e | Oracle ID: 049d1fb2-cce7-40e7-a633-e84cf9782d89
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DONATE,
    oracle_id = "049d1fb2-cce7-40e7-a633-e84cf9782d89",
    scryfall_id = "7f6d8ce9-f8c8-45ad-b74c-97fba0e2982e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Donate",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
