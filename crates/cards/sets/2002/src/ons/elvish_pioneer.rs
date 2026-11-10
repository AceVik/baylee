//! Elvish Pioneer — {G} — Creature — Elf Druid
//! Oracle: When this creature enters, you may put a basic land card from your hand onto the battlefield tapped.
//! Set: 8ED #243 — Eighth Edition | Scryfall ID: 88244522-1cf3-40a6-9ff7-bb78f2151b60 | Oracle ID: 5f4138e9-5471-467f-a15e-89e9e7929f0c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELVISH_PIONEER,
    oracle_id = "5f4138e9-5471-467f-a15e-89e9e7929f0c",
    scryfall_id = "88244522-1cf3-40a6-9ff7-bb78f2151b60",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Elvish Pioneer",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
