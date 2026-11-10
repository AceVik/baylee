//! Doomsday Specter — {2}{U}{B} — Creature — Specter
//! Oracle: Flying
//! Oracle: When this creature enters, return a blue or black creature you control to its owner's hand.
//! Oracle: Whenever this creature deals combat damage to a player, look at that player's hand and choose a card from it. The player discards that card.
//! Set: PLS #103 — Planeshift | Scryfall ID: 85206cc1-5484-40c6-b11d-b8d6fad4fc5c | Oracle ID: 5811effb-5010-44b9-bb97-9d7eb358327f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DOOMSDAY_SPECTER,
    oracle_id = "5811effb-5010-44b9-bb97-9d7eb358327f",
    scryfall_id = "85206cc1-5484-40c6-b11d-b8d6fad4fc5c",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(
        name = "Doomsday Specter",
        mana_cost = mana!("{2}{U}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPECTER],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
