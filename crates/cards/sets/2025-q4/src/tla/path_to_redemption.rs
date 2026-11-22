//! Path to Redemption — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't attack or block.
//! Oracle: {5}, Sacrifice this Aura: Exile enchanted creature. Create a 1/1 white Ally creature token. Activate only during your turn.
//! Set: TLA #31 — Avatar: The Last Airbender | Scryfall ID: f936d64a-0db4-49c1-8a57-6d99e012a555 | Oracle ID: ac94860b-70b8-4624-9758-4bb024a56f60
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PATH_TO_REDEMPTION,
    oracle_id = "ac94860b-70b8-4624-9758-4bb024a56f60",
    scryfall_id = "f936d64a-0db4-49c1-8a57-6d99e012a555",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Path to Redemption",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
