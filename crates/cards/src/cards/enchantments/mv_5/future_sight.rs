//! Future Sight — {2}{U}{U}{U} — Enchantment
//! Oracle: Play with the top card of your library revealed.
//! Oracle: You may play lands and cast spells from the top of your library.
//! Set: MH1 #53 — Modern Horizons | Scryfall ID: 639cb483-ffe2-4abc-a050-5cdc8aebd5a2 | Oracle ID: 93a0e938-b916-47ec-9803-369e885e253c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FUTURE_SIGHT,
    oracle_id = "93a0e938-b916-47ec-9803-369e885e253c",
    scryfall_id = "639cb483-ffe2-4abc-a050-5cdc8aebd5a2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Future Sight",
        mana_cost = mana!("{2}{U}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
