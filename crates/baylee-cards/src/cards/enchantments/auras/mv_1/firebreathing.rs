//! Firebreathing — {R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: {R}: Enchanted creature gets +1/+0 until end of turn.
//! Set: M12 #132 — Magic 2012 | Scryfall ID: 6fbcc269-aaa3-42aa-9898-3f908aaae272 | Oracle ID: 8603bf74-faab-4910-8e45-0f2e3b318efb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FIREBREATHING,
    oracle_id = "8603bf74-faab-4910-8e45-0f2e3b318efb",
    scryfall_id = "6fbcc269-aaa3-42aa-9898-3f908aaae272",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Firebreathing",
        mana_cost = mana!("{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
