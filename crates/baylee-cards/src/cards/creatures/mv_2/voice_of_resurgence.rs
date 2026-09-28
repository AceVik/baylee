//! Voice of Resurgence — {G}{W} — Creature — Elemental
//! Oracle: Whenever an opponent casts a spell during your turn and when this creature dies, create a green and white Elemental creature token with "This token's power and toughness are each equal to the number of creatures you control."
//! Set: 2XM #227 — Double Masters | Scryfall ID: 99d1e843-71c9-4a65-bc36-d23858ef5ead | Oracle ID: 5cbbb3f3-63a4-4983-81ea-8c405b10e63f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VOICE_OF_RESURGENCE,
    oracle_id = "5cbbb3f3-63a4-4983-81ea-8c405b10e63f",
    scryfall_id = "99d1e843-71c9-4a65-bc36-d23858ef5ead",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Voice of Resurgence",
        mana_cost = mana!("{G}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
