//! Rowen — {2}{G}{G} — Enchantment
//! Oracle: Reveal the first card you draw each turn. Whenever you reveal a basic land card this way, draw a card.
//! Set: 7ED #266 — Seventh Edition | Scryfall ID: bbc536ff-3e12-4e1b-b96f-b0c32dcb6734 | Oracle ID: eb44f5eb-1c6a-49f0-9751-23d167bf8288
// PARTIAL: enchantment on the battlefield; revealing the first card you draw each turn and drawing a card if it is a basic land is not yet supported.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Reveal the first card you draw each turn. Whenever you reveal a basic land card this way, draw a card.
card!(
    index = index::ROWEN,
    oracle_id = "eb44f5eb-1c6a-49f0-9751-23d167bf8288",
    scryfall_id = "bbc536ff-3e12-4e1b-b96f-b0c32dcb6734",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "enchantment on the battlefield; revealing the first card you draw each turn and drawing a card if it is a basic land is not yet supported"
    ),
    faces = &[face!(
        name = "Rowen",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);
