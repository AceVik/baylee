//! Gravestorm — {B}{B}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, target opponent may exile a card from their graveyard. If that player doesn't, you may draw a card.
//! Set: ODY #141 — Odyssey | Scryfall ID: 5e273b71-85ca-49d2-ba96-70cc0ae8d718 | Oracle ID: c815d510-73c9-4a8b-9947-f478a2f5297d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GRAVESTORM,
    oracle_id = "c815d510-73c9-4a8b-9947-f478a2f5297d",
    scryfall_id = "5e273b71-85ca-49d2-ba96-70cc0ae8d718",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Gravestorm",
        mana_cost = mana!("{B}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
