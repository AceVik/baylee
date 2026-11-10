//! Rowen — {2}{G}{G} — Enchantment
//! Oracle: Reveal the first card you draw each turn. Whenever you reveal a basic land card this way, draw a card.
//! Set: 7ED #266 — Seventh Edition | Scryfall ID: bbc536ff-3e12-4e1b-b96f-b0c32dcb6734 | Oracle ID: eb44f5eb-1c6a-49f0-9751-23d167bf8288
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ROWEN,
    oracle_id = "eb44f5eb-1c6a-49f0-9751-23d167bf8288",
    scryfall_id = "bbc536ff-3e12-4e1b-b96f-b0c32dcb6734",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Rowen",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
