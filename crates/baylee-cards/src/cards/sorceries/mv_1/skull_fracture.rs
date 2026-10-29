//! Skull Fracture — {B} — Sorcery
//! Oracle: Target player discards a card.
//! Oracle: Flashback {3}{B} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #162 — Odyssey | Scryfall ID: 93e3c013-319f-4a77-a55e-11323468b8ea | Oracle ID: 3384606b-7724-4584-89fa-fbe006d5de1f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SKULL_FRACTURE,
    oracle_id = "3384606b-7724-4584-89fa-fbe006d5de1f",
    scryfall_id = "93e3c013-319f-4a77-a55e-11323468b8ea",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Skull Fracture",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
