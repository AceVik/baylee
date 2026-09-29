//! Earthbind — {R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When this Aura enters, if enchanted creature has flying, this Aura deals 2 damage to that creature and this Aura gains "Enchanted creature loses flying."
//! Set: SUM #147 — Summer Magic / Edgar | Scryfall ID: 784cc686-e5a2-4835-8ff2-820461868ffa | Oracle ID: e8e35b49-8cfb-4fb5-89aa-8050f15b11bf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EARTHBIND,
    oracle_id = "e8e35b49-8cfb-4fb5-89aa-8050f15b11bf",
    scryfall_id = "784cc686-e5a2-4835-8ff2-820461868ffa",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Earthbind",
        mana_cost = mana!("{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
