//! Daru Spiritualist — {1}{W} — Creature — Human Cleric
//! Oracle: Whenever a Cleric creature you control becomes the target of a spell or ability, it gets +0/+2 until end of turn.
//! Set: SCG #5 — Scourge | Scryfall ID: 18f26b88-cffc-47ed-a70a-7d704a32c8bb | Oracle ID: e58a5631-dd00-4b1b-a2aa-2558bb0ff62f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DARU_SPIRITUALIST,
    oracle_id = "e58a5631-dd00-4b1b-a2aa-2558bb0ff62f",
    scryfall_id = "18f26b88-cffc-47ed-a70a-7d704a32c8bb",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Daru Spiritualist",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
