//! Gaea's Herald — {1}{G} — Creature — Elf
//! Oracle: Creature spells can't be countered.
//! Set: 10E #265 — Tenth Edition | Scryfall ID: 89a60271-edfc-4bb1-83d5-4d6fa0403b19 | Oracle ID: f21c3d22-6149-475f-ae6e-37a25ef020e5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GAEA_S_HERALD,
    oracle_id = "f21c3d22-6149-475f-ae6e-37a25ef020e5",
    scryfall_id = "89a60271-edfc-4bb1-83d5-4d6fa0403b19",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Gaea's Herald",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
