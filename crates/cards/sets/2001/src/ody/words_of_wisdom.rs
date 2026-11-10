//! Words of Wisdom — {1}{U} — Instant
//! Oracle: You draw two cards, then each other player draws a card.
//! Set: ODY #114 — Odyssey | Scryfall ID: c0199cfb-5e44-40f0-b9cf-71473155eb94 | Oracle ID: cff1b9bb-2a4c-4522-94a4-a674f942e376
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORDS_OF_WISDOM,
    oracle_id = "cff1b9bb-2a4c-4522-94a4-a674f942e376",
    scryfall_id = "c0199cfb-5e44-40f0-b9cf-71473155eb94",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Words of Wisdom",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
