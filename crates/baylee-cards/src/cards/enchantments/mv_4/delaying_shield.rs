//! Delaying Shield — {3}{W} — Enchantment
//! Oracle: If damage would be dealt to you, put that many delay counters on this enchantment instead.
//! Oracle: At the beginning of your upkeep, remove all delay counters from this enchantment. For each delay counter removed this way, you lose 1 life unless you pay {1}{W}.
//! Set: ODY #17 — Odyssey | Scryfall ID: 14e8195a-d0e4-4b11-b413-765ed1cae834 | Oracle ID: e03b553a-3d0d-453e-ad3e-f7d4b9fb2624
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DELAYING_SHIELD,
    oracle_id = "e03b553a-3d0d-453e-ad3e-f7d4b9fb2624",
    scryfall_id = "14e8195a-d0e4-4b11-b413-765ed1cae834",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Delaying Shield",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
