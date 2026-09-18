//! Sultai Charm — {B}{G}{U} — Instant
//! Oracle: Choose one —
//! Oracle: • Destroy target monocolored creature.
//! Oracle: • Destroy target artifact or enchantment.
//! Oracle: • Draw two cards, then discard a card.
//! Set: DMC #168 — Dominaria United Commander | Scryfall ID: 72af6c1f-33a0-4e02-95b7-74ecaa6a6d87 | Oracle ID: 46ed38d1-e642-4cea-99ed-a9c17fd982b1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SULTAI_CHARM,
    oracle_id = "46ed38d1-e642-4cea-99ed-a9c17fd982b1",
    scryfall_id = "72af6c1f-33a0-4e02-95b7-74ecaa6a6d87",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Blue]),
    faces = &[face!(
        name = "Sultai Charm",
        mana_cost = mana!("{B}{G}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
