//! Imperial Seal — {B} — Sorcery
//! Oracle: Search your library for a card, then shuffle and put that card on top. You lose 2 life.
//! Set: 2X2 #79 — Double Masters 2022 | Scryfall ID: e71a6bd3-7478-4c3a-8ae0-98352ce28492 | Oracle ID: 16cd0b90-f70c-4efa-b252-8de8784ef9a3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IMPERIAL_SEAL,
    oracle_id = "16cd0b90-f70c-4efa-b252-8de8784ef9a3",
    scryfall_id = "e71a6bd3-7478-4c3a-8ae0-98352ce28492",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Imperial Seal",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
