//! Repentant Vampire — {3}{B}{B} — Creature — Vampire
//! Oracle: Flying
//! Oracle: Whenever a creature dealt damage by this creature this turn dies, put a +1/+1 counter on this creature.
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature is white and has "{T}: Destroy target black creature."
//! Set: ODY #157 — Odyssey | Scryfall ID: 2c1dbad4-1105-4426-8d4d-a30f6fa95ce8 | Oracle ID: a5dabc98-0db9-4b4f-b2d2-f74dbd022d22
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::REPENTANT_VAMPIRE,
    oracle_id = "a5dabc98-0db9-4b4f-b2d2-f74dbd022d22",
    scryfall_id = "2c1dbad4-1105-4426-8d4d-a30f6fa95ce8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Repentant Vampire",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
