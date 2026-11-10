//! Dragon Fangs — {1}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+1 and has trample.
//! Oracle: When a creature with mana value 6 or greater enters, you may return this card from your graveyard to the battlefield attached to that creature.
//! Set: SCG #117 — Scourge | Scryfall ID: 9754f52f-8937-4402-8956-2c18b520898a | Oracle ID: 0131d2ac-d677-45fd-9729-a5428139cb5b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRAGON_FANGS,
    oracle_id = "0131d2ac-d677-45fd-9729-a5428139cb5b",
    scryfall_id = "9754f52f-8937-4402-8956-2c18b520898a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Dragon Fangs",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
