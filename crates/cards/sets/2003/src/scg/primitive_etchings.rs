//! Primitive Etchings — {2}{G}{G} — Enchantment
//! Oracle: Reveal the first card you draw each turn. Whenever you reveal a creature card this way, draw a card.
//! Set: SCG #126 — Scourge | Scryfall ID: eae26b8d-c3af-42d1-94f4-56950ceac1c7 | Oracle ID: dddda9a2-dc59-446d-bfc4-3240d12a3654
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRIMITIVE_ETCHINGS,
    oracle_id = "dddda9a2-dc59-446d-bfc4-3240d12a3654",
    scryfall_id = "eae26b8d-c3af-42d1-94f4-56950ceac1c7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Primitive Etchings",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
