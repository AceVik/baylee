//! Enduring Vitality — {1}{G}{G} — Enchantment Creature — Elk Glimmer
//! Oracle: Vigilance
//! Oracle: Creatures you control have "{T}: Add one mana of any color."
//! Oracle: When Enduring Vitality dies, if it was a creature, return it to the battlefield under its owner's control. It's an enchantment. (It's not a creature.)
//! Set: DSK #176 — Duskmourn: House of Horror | Scryfall ID: 9d76a30c-0431-4334-892a-9822dda9671a | Oracle ID: 3577c47e-76d3-4659-b922-31c4b74be3a0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENDURING_VITALITY,
    oracle_id = "3577c47e-76d3-4659-b922-31c4b74be3a0",
    scryfall_id = "9d76a30c-0431-4334-892a-9822dda9671a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Enduring Vitality",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::ELK, subtypes::creature::GLIMMER],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
