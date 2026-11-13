//! Cabal Conditioning — {6}{B} — Sorcery
//! Oracle: Any number of target players each discard a number of cards equal to the greatest mana value among permanents you control.
//! Set: SCG #56 — Scourge | Scryfall ID: eb81c6e6-fded-4cd3-a6fa-486419a5408a | Oracle ID: 37a04d5a-9272-42bd-b448-c658375bbbc3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CABAL_CONDITIONING,
    oracle_id = "37a04d5a-9272-42bd-b448-c658375bbbc3",
    scryfall_id = "eb81c6e6-fded-4cd3-a6fa-486419a5408a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cabal Conditioning",
        mana_cost = mana!("{6}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
