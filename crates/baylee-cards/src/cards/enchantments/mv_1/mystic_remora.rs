//! Mystic Remora — {U} — Enchantment
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Whenever an opponent casts a noncreature spell, you may draw a card unless that player pays {4}.
//! Set: DMR #59 — Dominaria Remastered | Scryfall ID: 40140991-cffa-4b52-9a25-37e9a8aa9ddd | Oracle ID: 8a52f3c0-2552-4425-b2e3-5496eb2232a7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MYSTIC_REMORA,
    oracle_id = "8a52f3c0-2552-4425-b2e3-5496eb2232a7",
    scryfall_id = "40140991-cffa-4b52-9a25-37e9a8aa9ddd",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mystic Remora",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
