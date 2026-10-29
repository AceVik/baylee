//! Weathered Wayfarer — {W} — Creature — Human Nomad Cleric
//! Oracle: {W}, {T}: Search your library for a land card, reveal it, put it into your hand, then shuffle. Activate only if an opponent controls more lands than you.
//! Set: 2X2 #34 — Double Masters 2022 | Scryfall ID: 09656d96-c366-49ec-b687-cad903de1385 | Oracle ID: 1093fdb6-bd4f-46a8-92e3-5aa46b52bb4f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WEATHERED_WAYFARER,
    oracle_id = "1093fdb6-bd4f-46a8-92e3-5aa46b52bb4f",
    scryfall_id = "09656d96-c366-49ec-b687-cad903de1385",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Weathered Wayfarer",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::NOMAD,
            subtypes::creature::CLERIC
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
