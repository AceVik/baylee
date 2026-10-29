//! Crown of Awe — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has protection from black and from red.
//! Oracle: Sacrifice this Aura: Enchanted creature and other creatures that share a creature type with it gain protection from black and from red until end of turn.
//! Set: ONS #16 — Onslaught | Scryfall ID: aeaea4bc-dcea-4340-a039-ebc97b944673 | Oracle ID: 602d0d09-00c6-4073-80ec-2ad33aef1a97
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CROWN_OF_AWE,
    oracle_id = "602d0d09-00c6-4073-80ec-2ad33aef1a97",
    scryfall_id = "aeaea4bc-dcea-4340-a039-ebc97b944673",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Crown of Awe",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
