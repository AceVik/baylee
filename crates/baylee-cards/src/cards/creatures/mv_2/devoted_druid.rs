//! Devoted Druid — {1}{G} — Creature — Elf Druid
//! Oracle: {T}: Add {G}.
//! Oracle: Put a -1/-1 counter on this creature: Untap this creature.
//! Set: ECC #104 — Lorwyn Eclipsed Commander | Scryfall ID: 22589a81-3ea8-4e78-98c9-c015e7539cf9 | Oracle ID: cb814e16-acf7-41d5-a357-1323dcc369f3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEVOTED_DRUID,
    oracle_id = "cb814e16-acf7-41d5-a357-1323dcc369f3",
    scryfall_id = "22589a81-3ea8-4e78-98c9-c015e7539cf9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Devoted Druid",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::DRUID],
        power = Some(0),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
