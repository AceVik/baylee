//! Sleeping Potion — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When this Aura enters, tap enchanted creature.
//! Oracle: Enchanted creature doesn't untap during its controller's untap step.
//! Oracle: When enchanted creature becomes the target of a spell or ability, sacrifice this Aura.
//! Set: PLS #34 — Planeshift | Scryfall ID: 6f79f4b2-71cd-4f78-a161-d75b162c745e | Oracle ID: c8fbcef0-2cbf-4a59-8322-374dd9153ebf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SLEEPING_POTION,
    oracle_id = "c8fbcef0-2cbf-4a59-8322-374dd9153ebf",
    scryfall_id = "6f79f4b2-71cd-4f78-a161-d75b162c745e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Sleeping Potion",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
