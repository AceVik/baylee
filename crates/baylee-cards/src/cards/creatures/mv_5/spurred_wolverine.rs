//! Spurred Wolverine — {4}{R} — Creature — Wolverine Beast
//! Oracle: Tap two untapped Beasts you control: Target creature gains first strike until end of turn.
//! Set: ONS #237 — Onslaught | Scryfall ID: 46d7aaea-226b-4820-8db2-89dcdcbcc557 | Oracle ID: 245b62a8-b606-4106-87c3-a60ce3e16906
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPURRED_WOLVERINE,
    oracle_id = "245b62a8-b606-4106-87c3-a60ce3e16906",
    scryfall_id = "46d7aaea-226b-4820-8db2-89dcdcbcc557",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Spurred Wolverine",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WOLVERINE, subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
