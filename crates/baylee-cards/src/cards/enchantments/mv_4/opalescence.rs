//! Opalescence — {2}{W}{W} — Enchantment
//! Oracle: Each other non-Aura enchantment is a creature in addition to its other types and has base power and base toughness each equal to its mana value.
//! Set: UDS #13 — Urza's Destiny | Scryfall ID: 3c0071fb-afa5-47b5-b266-2b10a4f5a98a | Oracle ID: 59489b46-9d02-4f3c-bcd0-884e7605e9a5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OPALESCENCE,
    oracle_id = "59489b46-9d02-4f3c-bcd0-884e7605e9a5",
    scryfall_id = "3c0071fb-afa5-47b5-b266-2b10a4f5a98a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Opalescence",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
