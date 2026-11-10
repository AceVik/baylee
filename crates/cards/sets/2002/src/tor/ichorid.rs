//! Ichorid — {3}{B} — Creature — Horror
//! Oracle: Haste
//! Oracle: At the beginning of the end step, sacrifice this creature.
//! Oracle: At the beginning of your upkeep, if this card is in your graveyard, you may exile a black creature card other than this card from your graveyard. If you do, return this card to the battlefield.
//! Set: EMA #93 — Eternal Masters | Scryfall ID: cd5bc1e4-eefb-4613-aa18-2bf2b9bdf415 | Oracle ID: 5261ba70-b641-461d-9d9f-fce17764ad67
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ICHORID,
    oracle_id = "5261ba70-b641-461d-9d9f-fce17764ad67",
    scryfall_id = "cd5bc1e4-eefb-4613-aa18-2bf2b9bdf415",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Ichorid",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(3),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
