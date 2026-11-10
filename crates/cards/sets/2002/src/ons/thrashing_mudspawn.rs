//! Thrashing Mudspawn — {3}{B}{B} — Creature — Beast
//! Oracle: Whenever this creature is dealt damage, you lose that much life.
//! Oracle: Morph {1}{B}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #177 — Onslaught | Scryfall ID: da84de0e-a4cd-4dff-8ee3-87c9debf0969 | Oracle ID: 7f67b8e0-fbbd-48ef-989d-690d44b70304
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THRASHING_MUDSPAWN,
    oracle_id = "7f67b8e0-fbbd-48ef-989d-690d44b70304",
    scryfall_id = "da84de0e-a4cd-4dff-8ee3-87c9debf0969",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Thrashing Mudspawn",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
