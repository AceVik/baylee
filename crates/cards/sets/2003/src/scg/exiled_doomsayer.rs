//! Exiled Doomsayer — {1}{W} — Creature — Human Cleric
//! Oracle: All morph costs cost {2} more. (This doesn't affect the cost to cast creature spells face down.)
//! Set: SCG #13 — Scourge | Scryfall ID: 74aaf095-143a-43fc-a858-b1e82a4b906e | Oracle ID: 7a4c2d47-3489-4e56-b38b-de2dddd942ad
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EXILED_DOOMSAYER,
    oracle_id = "7a4c2d47-3489-4e56-b38b-de2dddd942ad",
    scryfall_id = "74aaf095-143a-43fc-a858-b1e82a4b906e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Exiled Doomsayer",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
