//! Squee's Embrace — {R}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/+2.
//! Oracle: When enchanted creature dies, return that card to its owner's hand.
//! Set: APC #122 — Apocalypse | Scryfall ID: e682a705-9341-4d1e-a9c5-d428e50b9a03 | Oracle ID: 095f8dac-15b8-4c28-ae33-6dc71152bcc7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SQUEE_S_EMBRACE,
    oracle_id = "095f8dac-15b8-4c28-ae33-6dc71152bcc7",
    scryfall_id = "e682a705-9341-4d1e-a9c5-d428e50b9a03",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[face!(
        name = "Squee's Embrace",
        mana_cost = mana!("{R}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
