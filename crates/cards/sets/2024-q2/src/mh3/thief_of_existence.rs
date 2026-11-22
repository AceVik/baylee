//! Thief of Existence — {1}{C}{G} — Creature — Eldrazi
//! Oracle: Devoid (This card has no color.)
//! Oracle: When you cast this spell, exile up to one target noncreature, nonland permanent an opponent controls with mana value 4 or less. If you do, Thief of Existence gains "When this creature leaves the battlefield, target opponent draws a card."
//! Set: MH3 #174 — Modern Horizons 3 | Scryfall ID: 328b02ca-d8eb-401d-9c41-93f8eb909312 | Oracle ID: 7abbd5af-a04b-476d-aee5-2b2f5114b019
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THIEF_OF_EXISTENCE,
    oracle_id = "7abbd5af-a04b-476d-aee5-2b2f5114b019",
    scryfall_id = "328b02ca-d8eb-401d-9c41-93f8eb909312",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Thief of Existence",
        mana_cost = mana!("{1}{C}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELDRAZI],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
