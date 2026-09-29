//! Lifeforce — {G}{G} — Enchantment
//! Oracle: {G}{G}: Counter target black spell.
//! Set: ME4 #160 — Masters Edition IV | Scryfall ID: 6520d992-ebb3-4711-ad91-ab1c1e29538a | Oracle ID: 07ae1fe5-5c3e-4d94-b809-8defd2ef44e3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIFEFORCE,
    oracle_id = "07ae1fe5-5c3e-4d94-b809-8defd2ef44e3",
    scryfall_id = "6520d992-ebb3-4711-ad91-ab1c1e29538a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Lifeforce",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
