//! Farewell — {4}{W}{W} — Sorcery
//! Oracle: Choose one or more —
//! Oracle: • Exile all artifacts.
//! Oracle: • Exile all creatures.
//! Oracle: • Exile all enchantments.
//! Oracle: • Exile all graveyards.
//! Set: MKC #64 — Murders at Karlov Manor Commander | Scryfall ID: 114d2180-093b-4838-97ad-badbc8ee50b0 | Oracle ID: 4eb813fd-2d5a-4b02-8193-662681ef4e7d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FAREWELL,
    oracle_id = "4eb813fd-2d5a-4b02-8193-662681ef4e7d",
    scryfall_id = "114d2180-093b-4838-97ad-badbc8ee50b0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Farewell",
        mana_cost = mana!("{4}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
