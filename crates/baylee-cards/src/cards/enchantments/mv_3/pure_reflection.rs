//! Pure Reflection — {2}{W} — Enchantment
//! Oracle: Whenever a player casts a creature spell, destroy all Reflections. Then that player creates an X/X white Reflection creature token, where X is the mana value of that spell.
//! Set: INV #27 — Invasion | Scryfall ID: bbff85a6-a51b-424e-a86b-da52c9b3a9da | Oracle ID: fee53820-a17c-4f34-9971-581bf8e12fd6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PURE_REFLECTION,
    oracle_id = "fee53820-a17c-4f34-9971-581bf8e12fd6",
    scryfall_id = "bbff85a6-a51b-424e-a86b-da52c9b3a9da",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Pure Reflection",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
