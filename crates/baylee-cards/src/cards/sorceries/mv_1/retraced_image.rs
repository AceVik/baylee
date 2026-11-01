//! Retraced Image — {U} — Sorcery
//! Oracle: Reveal a card in your hand, then put that card onto the battlefield if it has the same name as a permanent.
//! Set: TOR #46 — Torment | Scryfall ID: c62fa778-ce44-4a3e-958a-91dfe5398944 | Oracle ID: a97f2f2a-5a8c-4f8a-99d1-83ebc4b0bdba
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RETRACED_IMAGE,
    oracle_id = "a97f2f2a-5a8c-4f8a-99d1-83ebc4b0bdba",
    scryfall_id = "c62fa778-ce44-4a3e-958a-91dfe5398944",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Retraced Image",
        mana_cost = mana!("{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
