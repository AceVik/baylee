//! Molten Influence — {1}{R} — Instant
//! Oracle: Counter target instant or sorcery spell unless its controller has Molten Influence deal 4 damage to them.
//! Set: ODY #207 — Odyssey | Scryfall ID: 4c2b326b-d177-4a03-a0a3-fe2c2d4af272 | Oracle ID: d955a26a-d474-4dff-9eba-69fcfe92d8ae
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOLTEN_INFLUENCE,
    oracle_id = "d955a26a-d474-4dff-9eba-69fcfe92d8ae",
    scryfall_id = "4c2b326b-d177-4a03-a0a3-fe2c2d4af272",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Molten Influence",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
