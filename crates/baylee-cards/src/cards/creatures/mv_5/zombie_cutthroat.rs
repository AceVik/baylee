//! Zombie Cutthroat — {3}{B}{B} — Creature — Zombie
//! Oracle: Morph—Pay 5 life. (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: SCG #81 — Scourge | Scryfall ID: fe491cba-6ec7-4c44-ad1e-832d936986a0 | Oracle ID: a0b6aa7d-a8cd-4af7-8fb2-d2fa0861e123
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZOMBIE_CUTTHROAT,
    oracle_id = "a0b6aa7d-a8cd-4af7-8fb2-d2fa0861e123",
    scryfall_id = "fe491cba-6ec7-4c44-ad1e-832d936986a0",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Zombie Cutthroat",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
