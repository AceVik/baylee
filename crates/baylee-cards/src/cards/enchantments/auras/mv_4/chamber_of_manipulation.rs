//! Chamber of Manipulation — {2}{U}{U} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has "{T}, Discard a card: Gain control of target creature until end of turn."
//! Set: ODY #76 — Odyssey | Scryfall ID: 50496069-0f55-4dfe-8b1e-7fc4f649f2f1 | Oracle ID: 010e0d5c-e751-4f67-b305-8a598cc1f0de
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHAMBER_OF_MANIPULATION,
    oracle_id = "010e0d5c-e751-4f67-b305-8a598cc1f0de",
    scryfall_id = "50496069-0f55-4dfe-8b1e-7fc4f649f2f1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Chamber of Manipulation",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
