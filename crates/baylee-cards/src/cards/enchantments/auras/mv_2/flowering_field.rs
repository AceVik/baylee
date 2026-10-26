//! Flowering Field — {1}{W} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has "{T}: Prevent the next 1 damage that would be dealt to any target this turn."
//! Set: PCY #9 — Prophecy | Scryfall ID: c241fd76-f52d-48fc-864c-57caffa700f6 | Oracle ID: 40155cfb-078a-4359-9551-9c71266d8a1c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLOWERING_FIELD,
    oracle_id = "40155cfb-078a-4359-9551-9c71266d8a1c",
    scryfall_id = "c241fd76-f52d-48fc-864c-57caffa700f6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Flowering Field",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
