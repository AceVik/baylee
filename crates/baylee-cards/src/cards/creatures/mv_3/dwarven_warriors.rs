//! Dwarven Warriors — {2}{R} — Creature — Dwarf Warrior
//! Oracle: {T}: Target creature with power 2 or less can't be blocked this turn.
//! Set: 5ED #222 — Fifth Edition | Scryfall ID: d217942b-1253-4d4b-b0d2-09fc4e15cc62 | Oracle ID: cfc553cd-3b4c-47a9-bffb-e5790befb32c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DWARVEN_WARRIORS,
    oracle_id = "cfc553cd-3b4c-47a9-bffb-e5790befb32c",
    scryfall_id = "d217942b-1253-4d4b-b0d2-09fc4e15cc62",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dwarven Warriors",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF, subtypes::creature::WARRIOR],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
