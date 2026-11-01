//! Pox — {B}{B}{B} — Sorcery
//! Oracle: Each player loses a third of their life, then discards a third of the cards in their hand, then sacrifices a third of the creatures they control of their choice, then sacrifices a third of the lands they control of their choice. Round up each time.
//! Set: ME1 #82 — Masters Edition | Scryfall ID: d9debe4f-c1cc-4b90-aae4-f31f737e2016 | Oracle ID: 539d7c1a-b5e9-4204-9109-ddb2e8dec168
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::POX,
    oracle_id = "539d7c1a-b5e9-4204-9109-ddb2e8dec168",
    scryfall_id = "d9debe4f-c1cc-4b90-aae4-f31f737e2016",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Pox",
        mana_cost = mana!("{B}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
