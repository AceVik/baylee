//! Carrion Wurm — {3}{B}{B} — Creature — Zombie Wurm
//! Oracle: Whenever this creature attacks or blocks, any player may exile three cards from their graveyard. If a player does, this creature assigns no combat damage this turn.
//! Set: TOR #55 — Torment | Scryfall ID: 37c2b228-94c0-4e84-ad6d-80b170bb6c0c | Oracle ID: ed0cf504-c365-4486-92bc-c329b83b99d0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CARRION_WURM,
    oracle_id = "ed0cf504-c365-4486-92bc-c329b83b99d0",
    scryfall_id = "37c2b228-94c0-4e84-ad6d-80b170bb6c0c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Carrion Wurm",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::WURM],
        power = Some(6),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
