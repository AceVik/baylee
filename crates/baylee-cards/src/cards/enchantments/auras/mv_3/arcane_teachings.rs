//! Arcane Teachings — {2}{R} — Enchantment — Aura
//! Oracle: Enchant creature (Target a creature as you cast this. This card enters attached to that creature.)
//! Oracle: Enchanted creature gets +2/+2 and has "{T}: This creature deals 1 damage to any target."
//! Set: 10E #188 — Tenth Edition | Scryfall ID: 41764237-fa31-4b27-92ff-712a42d350b9 | Oracle ID: 0b633017-bfdf-47cd-a60f-c93816b47e9c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARCANE_TEACHINGS,
    oracle_id = "0b633017-bfdf-47cd-a60f-c93816b47e9c",
    scryfall_id = "41764237-fa31-4b27-92ff-712a42d350b9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Arcane Teachings",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
