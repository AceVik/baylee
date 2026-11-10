//! Dragon Breath — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has haste.
//! Oracle: {R}: Enchanted creature gets +1/+0 until end of turn.
//! Oracle: When a creature with mana value 6 or greater enters, you may return this card from your graveyard to the battlefield attached to that creature.
//! Set: BBD #172 — Battlebond | Scryfall ID: 0900c3b6-cf56-4089-a1e3-c1f53edcf23a | Oracle ID: 3535daf2-7dce-4fda-92ce-ad2a1ee95197
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRAGON_BREATH,
    oracle_id = "3535daf2-7dce-4fda-92ce-ad2a1ee95197",
    scryfall_id = "0900c3b6-cf56-4089-a1e3-c1f53edcf23a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dragon Breath",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
