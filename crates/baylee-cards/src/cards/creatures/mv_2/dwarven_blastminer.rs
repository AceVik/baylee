//! Dwarven Blastminer — {1}{R} — Creature — Dwarf
//! Oracle: {2}{R}, {T}: Destroy target nonbasic land.
//! Oracle: Morph {R} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #199 — Onslaught | Scryfall ID: 2970831a-738b-476f-9d46-39f10a1f91e7 | Oracle ID: b373e978-c58b-465a-b68a-9d4acec9dce0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DWARVEN_BLASTMINER,
    oracle_id = "b373e978-c58b-465a-b68a-9d4acec9dce0",
    scryfall_id = "2970831a-738b-476f-9d46-39f10a1f91e7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Dwarven Blastminer",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
