//! Fear — {B}{B} — Enchantment — Aura
//! Oracle: Enchant creature (Target a creature as you cast this. This card enters attached to that creature.)
//! Oracle: Enchanted creature has fear. (It can't be blocked except by artifact creatures and/or black creatures.)
//! Set: 10E #142 — Tenth Edition | Scryfall ID: d8fa57eb-e307-4104-a291-c6cbfa235816 | Oracle ID: 355bbe9b-59bf-470f-8600-410af4c7fe18
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FEAR,
    oracle_id = "355bbe9b-59bf-470f-8600-410af4c7fe18",
    scryfall_id = "d8fa57eb-e307-4104-a291-c6cbfa235816",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Fear",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
