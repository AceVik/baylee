//! Harvest Mage — {G} — Creature — Human Spellshaper
//! Oracle: {G}, {T}, Discard a card: Until end of turn, if you tap a land for mana, it produces one mana of a color of your choice instead of any other type and amount.
//! Set: NEM #105 — Nemesis | Scryfall ID: 95b29329-b9a3-4d59-b0f8-2abc67337760 | Oracle ID: 6f82b01d-8975-42a1-87e2-99782272bbea
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HARVEST_MAGE,
    oracle_id = "6f82b01d-8975-42a1-87e2-99782272bbea",
    scryfall_id = "95b29329-b9a3-4d59-b0f8-2abc67337760",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Harvest Mage",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SPELLSHAPER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
