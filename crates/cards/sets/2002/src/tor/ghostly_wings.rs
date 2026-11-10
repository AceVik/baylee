//! Ghostly Wings — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+1 and has flying.
//! Oracle: Discard a card: Return enchanted creature to its owner's hand.
//! Set: SOI #66 — Shadows over Innistrad | Scryfall ID: f1b1848c-3160-45f5-91a0-74ab6d91c01b | Oracle ID: b07d8095-c5e2-4bf8-b055-af97ce5df4a4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GHOSTLY_WINGS,
    oracle_id = "b07d8095-c5e2-4bf8-b055-af97ce5df4a4",
    scryfall_id = "f1b1848c-3160-45f5-91a0-74ab6d91c01b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Ghostly Wings",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
