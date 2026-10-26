//! Heightened Awareness — {3}{U}{U} — Enchantment
//! Oracle: As this enchantment enters, discard your hand.
//! Oracle: At the beginning of your draw step, draw an additional card.
//! Set: PCY #37 — Prophecy | Scryfall ID: 2765be4f-23bf-49c1-9546-11a7916156be | Oracle ID: a69a0720-35fb-450d-8d48-8e51a14ffa49
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HEIGHTENED_AWARENESS,
    oracle_id = "a69a0720-35fb-450d-8d48-8e51a14ffa49",
    scryfall_id = "2765be4f-23bf-49c1-9546-11a7916156be",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Heightened Awareness",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
