//! Putrid Raptor — {4}{B}{B} — Creature — Zombie Dinosaur Beast
//! Oracle: Morph—Discard a Zombie card. (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: SCG #71 — Scourge | Scryfall ID: 9127942b-d73d-42a9-9f97-6a39fa798a8b | Oracle ID: 31b43ad8-a20b-4948-bfd2-56d7fe1a78f0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PUTRID_RAPTOR,
    oracle_id = "31b43ad8-a20b-4948-bfd2-56d7fe1a78f0",
    scryfall_id = "9127942b-d73d-42a9-9f97-6a39fa798a8b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Putrid Raptor",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::ZOMBIE,
            subtypes::creature::DINOSAUR,
            subtypes::creature::BEAST
        ],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
