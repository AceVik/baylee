//! Timberwatch Elf — {2}{G} — Creature — Elf
//! Oracle: {T}: Target creature gets +X/+X until end of turn, where X is the number of Elves on the battlefield.
//! Set: KHC #76 — Kaldheim Commander | Scryfall ID: 38807f17-1cf2-4736-ad10-df6c8b1a9f55 | Oracle ID: 50cee3ac-cba0-4abb-babf-de1928b1590e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TIMBERWATCH_ELF,
    oracle_id = "50cee3ac-cba0-4abb-babf-de1928b1590e",
    scryfall_id = "38807f17-1cf2-4736-ad10-df6c8b1a9f55",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Timberwatch Elf",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
