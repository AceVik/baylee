//! Ceta Sanctuary — {2}{U} — Enchantment
//! Oracle: At the beginning of your upkeep, if you control a red or green permanent, draw a card, then discard a card. If you control a red permanent and a green permanent, instead draw two cards, then discard a card.
//! Set: APC #20 — Apocalypse | Scryfall ID: 32cec6f3-295a-45e3-8466-e35fb043a596 | Oracle ID: 45c4e67c-e452-41c5-8baa-050819a1321f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CETA_SANCTUARY,
    oracle_id = "45c4e67c-e452-41c5-8baa-050819a1321f",
    scryfall_id = "32cec6f3-295a-45e3-8466-e35fb043a596",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Ceta Sanctuary",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
