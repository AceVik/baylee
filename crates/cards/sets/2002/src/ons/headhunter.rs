//! Headhunter — {1}{B} — Creature — Human Cleric
//! Oracle: Whenever this creature deals combat damage to a player, that player discards a card.
//! Oracle: Morph {B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #156 — Onslaught | Scryfall ID: 3cbd82d5-d64f-4833-b1a9-9652fcfa1578 | Oracle ID: 83578de7-5488-4952-af47-8fe966897210
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HEADHUNTER,
    oracle_id = "83578de7-5488-4952-af47-8fe966897210",
    scryfall_id = "3cbd82d5-d64f-4833-b1a9-9652fcfa1578",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Headhunter",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
