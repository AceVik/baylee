//! Tidal Courier — {3}{U} — Creature — Merfolk
//! Oracle: When this creature enters, reveal the top four cards of your library. Put all Merfolk cards revealed this way into your hand and the rest on the bottom of your library in any order.
//! Oracle: {3}{U}: This creature gains flying until end of turn.
//! Set: DDT #19 — Duel Decks: Merfolk vs. Goblins | Scryfall ID: 39692846-0819-42a3-af25-d9e3a4233b52 | Oracle ID: b0a0c17b-87c8-4610-adca-5e7f5ced49f6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TIDAL_COURIER,
    oracle_id = "b0a0c17b-87c8-4610-adca-5e7f5ced49f6",
    scryfall_id = "39692846-0819-42a3-af25-d9e3a4233b52",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Tidal Courier",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
