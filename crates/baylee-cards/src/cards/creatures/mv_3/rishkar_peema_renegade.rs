//! Rishkar, Peema Renegade — {2}{G} — Legendary Creature — Elf Druid
//! Oracle: When Rishkar enters, put a +1/+1 counter on each of up to two target creatures.
//! Oracle: Each creature you control with a counter on it has "{T}: Add {G}."
//! Set: CMM #317 — Commander Masters | Scryfall ID: 88f4aecc-b728-4960-8131-1e5aa6c3c030 | Oracle ID: 761021ce-4559-464e-aa03-85c2fe78e267
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RISHKAR_PEEMA_RENEGADE,
    oracle_id = "761021ce-4559-464e-aa03-85c2fe78e267",
    scryfall_id = "88f4aecc-b728-4960-8131-1e5aa6c3c030",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Rishkar, Peema Renegade",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::DRUID],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
