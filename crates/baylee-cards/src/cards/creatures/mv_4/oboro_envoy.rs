//! Oboro Envoy — {3}{U} — Creature — Moonfolk Wizard
//! Oracle: Flying
//! Oracle: {2}, Return a land you control to its owner's hand: Target creature gets -X/-0 until end of turn, where X is the number of cards in your hand.
//! Set: SOK #49 — Saviors of Kamigawa | Scryfall ID: a0309dcb-6c13-4ffe-b44d-3b735c8277d2 | Oracle ID: be70c6e8-6f9f-49fb-ab40-c6ce0ec2077c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OBORO_ENVOY,
    oracle_id = "be70c6e8-6f9f-49fb-ab40-c6ce0ec2077c",
    scryfall_id = "a0309dcb-6c13-4ffe-b44d-3b735c8277d2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Oboro Envoy",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MOONFOLK, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
