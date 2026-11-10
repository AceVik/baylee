//! Ebonblade Reaper — {2}{B} — Creature — Human Cleric
//! Oracle: Whenever this creature attacks, you lose half your life, rounded up.
//! Oracle: Whenever this creature deals combat damage to a player, that player loses half their life, rounded up.
//! Oracle: Morph {3}{B}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #141 — Onslaught | Scryfall ID: 16ebef2c-8bb2-4816-a628-0062f95e512e | Oracle ID: 86fd4652-a629-429a-bf95-0b8724708a71
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EBONBLADE_REAPER,
    oracle_id = "86fd4652-a629-429a-bf95-0b8724708a71",
    scryfall_id = "16ebef2c-8bb2-4816-a628-0062f95e512e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Ebonblade Reaper",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
