//! Samite Healer — {1}{W} — Creature — Human Cleric
//! Oracle: {T}: Prevent the next 1 damage that would be dealt to any target this turn.
//! Set: 10E #38 — Tenth Edition | Scryfall ID: 621f9a58-0bc8-40dd-aef1-43618274d6fe | Oracle ID: 95a0ca48-d924-47f4-86ed-42c673ee778c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAMITE_HEALER,
    oracle_id = "95a0ca48-d924-47f4-86ed-42c673ee778c",
    scryfall_id = "621f9a58-0bc8-40dd-aef1-43618274d6fe",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Samite Healer",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
