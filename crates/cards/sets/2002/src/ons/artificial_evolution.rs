//! Artificial Evolution — {U} — Instant
//! Oracle: Change the text of target spell or permanent by replacing all instances of one creature type with another. The new creature type can't be Wall. (This effect lasts indefinitely.)
//! Set: ONS #67 — Onslaught | Scryfall ID: f46894d1-2503-43fa-938e-7bbf19101d13 | Oracle ID: c5ab6dc9-34af-434f-ae8e-b2389ee6c5ae
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARTIFICIAL_EVOLUTION,
    oracle_id = "c5ab6dc9-34af-434f-ae8e-b2389ee6c5ae",
    scryfall_id = "f46894d1-2503-43fa-938e-7bbf19101d13",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Artificial Evolution",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
