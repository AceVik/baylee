//! Infected Vermin — {2}{B} — Creature — Rat
//! Oracle: {2}{B}: This creature deals 1 damage to each creature and each player.
//! Oracle: Threshold — {3}{B}: This creature deals 3 damage to each creature and each player. Activate only if there are seven or more cards in your graveyard.
//! Set: ODY #144 — Odyssey | Scryfall ID: 00615487-3526-4b4c-bb06-8f2af1f101d0 | Oracle ID: 448396b8-b691-457a-8569-1282c3fca65a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INFECTED_VERMIN,
    oracle_id = "448396b8-b691-457a-8569-1282c3fca65a",
    scryfall_id = "00615487-3526-4b4c-bb06-8f2af1f101d0",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Infected Vermin",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::RAT],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
