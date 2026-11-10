//! Synapse Sliver — {4}{U} — Creature — Sliver
//! Oracle: Whenever a Sliver deals combat damage to a player, its controller may draw a card.
//! Set: CMM #857 — Commander Masters | Scryfall ID: 7c458a0f-0e63-417f-b51e-8491822835b0 | Oracle ID: b76f123d-7b78-42a1-a66e-0dee6f773687
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SYNAPSE_SLIVER,
    oracle_id = "b76f123d-7b78-42a1-a66e-0dee6f773687",
    scryfall_id = "7c458a0f-0e63-417f-b51e-8491822835b0",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Synapse Sliver",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
