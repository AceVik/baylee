//! Nature's Kiss — {1}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: {1}, Exile the top card of your graveyard: Enchanted creature gets +1/+1 until end of turn.
//! Set: WTH #136 — Weatherlight | Scryfall ID: 64b09c44-d463-45a9-9fa2-89407c21200b | Oracle ID: e3e87fee-5717-4ca1-adfd-4df8732020f8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NATURE_S_KISS,
    oracle_id = "e3e87fee-5717-4ca1-adfd-4df8732020f8",
    scryfall_id = "64b09c44-d463-45a9-9fa2-89407c21200b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Nature's Kiss",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
