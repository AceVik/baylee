//! Keeper of the Nine Gales — {2}{U} — Creature — Bird Wizard
//! Oracle: Flying
//! Oracle: {T}, Tap two untapped Birds you control: Return target permanent to its owner's hand.
//! Set: LGN #42 — Legions | Scryfall ID: f75eef50-b474-44bb-8222-3e473928304a | Oracle ID: 39c4357b-6093-486b-8131-041a3f89af75
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KEEPER_OF_THE_NINE_GALES,
    oracle_id = "39c4357b-6093-486b-8131-041a3f89af75",
    scryfall_id = "f75eef50-b474-44bb-8222-3e473928304a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Keeper of the Nine Gales",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
