//! Robber Fly — {2}{R} — Creature — Insect
//! Oracle: Flying
//! Oracle: Whenever this creature becomes blocked, defending player discards all the cards in their hand, then draws that many cards.
//! Set: MMQ #209 — Mercadian Masques | Scryfall ID: 7d5cf073-2ba0-463e-bcd4-979ad18e28fc | Oracle ID: 0d44d4f3-9713-4d18-a985-9e7ff0df85d3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROBBER_FLY,
    oracle_id = "0d44d4f3-9713-4d18-a985-9e7ff0df85d3",
    scryfall_id = "7d5cf073-2ba0-463e-bcd4-979ad18e28fc",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Robber Fly",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
