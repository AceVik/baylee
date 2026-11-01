//! Laccolith Rig — {R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature becomes blocked, you may have it deal damage equal to its power to target creature. If you do, the first creature assigns no combat damage this turn.
//! Set: NEM #88 — Nemesis | Scryfall ID: 4fb92039-03fd-4aee-be74-96997be629d6 | Oracle ID: 0b583980-1508-4247-8a28-7127241d8ec0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LACCOLITH_RIG,
    oracle_id = "0b583980-1508-4247-8a28-7127241d8ec0",
    scryfall_id = "4fb92039-03fd-4aee-be74-96997be629d6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Laccolith Rig",
        mana_cost = mana!("{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
