//! Pernicious Deed — {1}{B}{G} — Enchantment
//! Oracle: {X}, Sacrifice this enchantment: Destroy each artifact, creature, and enchantment with mana value X or less.
//! Set: A25 #212 — Masters 25 | Scryfall ID: 8652e3a1-bcd4-4c0c-a085-34f19702df26 | Oracle ID: 62e44e0d-eda0-4275-8367-49dab9a087c3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PERNICIOUS_DEED,
    oracle_id = "62e44e0d-eda0-4275-8367-49dab9a087c3",
    scryfall_id = "8652e3a1-bcd4-4c0c-a085-34f19702df26",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Pernicious Deed",
        mana_cost = mana!("{1}{B}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
