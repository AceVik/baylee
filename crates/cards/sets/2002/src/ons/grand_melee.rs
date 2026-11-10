//! Grand Melee — {3}{R} — Enchantment
//! Oracle: All creatures attack each combat if able.
//! Oracle: All creatures block each combat if able.
//! Set: ONS #211 — Onslaught | Scryfall ID: 9a0d3142-4224-4b51-885d-33c8938418c1 | Oracle ID: 1accf98a-0905-4a9d-9ab3-72e9f853f4ab
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GRAND_MELEE,
    oracle_id = "1accf98a-0905-4a9d-9ab3-72e9f853f4ab",
    scryfall_id = "9a0d3142-4224-4b51-885d-33c8938418c1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Grand Melee",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
