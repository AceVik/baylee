//! Essence Fracture — {3}{U}{U} — Sorcery
//! Oracle: Return two target creatures to their owners' hands.
//! Oracle: Cycling {2}{U} ({2}{U}, Discard this card: Draw a card.)
//! Set: ONS #82 — Onslaught | Scryfall ID: df0b6c7a-0891-492d-8e07-6a198bf2ccc4 | Oracle ID: 31a49fb9-1081-46c3-a352-0444869afdc9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ESSENCE_FRACTURE,
    oracle_id = "31a49fb9-1081-46c3-a352-0444869afdc9",
    scryfall_id = "df0b6c7a-0891-492d-8e07-6a198bf2ccc4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Essence Fracture",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
