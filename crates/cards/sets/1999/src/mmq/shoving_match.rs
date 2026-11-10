//! Shoving Match — {2}{U} — Instant
//! Oracle: Until end of turn, all creatures gain "{T}: Tap target creature."
//! Set: MMQ #103 — Mercadian Masques | Scryfall ID: aa9f4787-9b29-4f57-b105-1f9eb4bb8861 | Oracle ID: 1ef07ae5-3b04-45c8-a080-852b84da4fef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHOVING_MATCH,
    oracle_id = "1ef07ae5-3b04-45c8-a080-852b84da4fef",
    scryfall_id = "aa9f4787-9b29-4f57-b105-1f9eb4bb8861",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Shoving Match",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
