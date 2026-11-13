//! Psychic Theft — {1}{U} — Sorcery
//! Oracle: Target player reveals their hand. You choose an instant or sorcery card from it and exile that card. You may cast that card for as long as it remains exiled. At the beginning of the next end step, if you haven't cast the card, return it to its owner's hand.
//! Set: PCY #40 — Prophecy | Scryfall ID: f6f86c92-c19e-4f2c-96d3-d3b05623cb00 | Oracle ID: 66953dfd-d908-41d3-abb4-309c6a165dc3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PSYCHIC_THEFT,
    oracle_id = "66953dfd-d908-41d3-abb4-309c6a165dc3",
    scryfall_id = "f6f86c92-c19e-4f2c-96d3-d3b05623cb00",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Psychic Theft",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
