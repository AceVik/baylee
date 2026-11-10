//! Entrails Feaster — {B} — Creature — Zombie Cat
//! Oracle: At the beginning of your upkeep, you may exile a creature card from a graveyard. If you do, put a +1/+1 counter on this creature. If you don't, tap this creature.
//! Set: ONS #143 — Onslaught | Scryfall ID: cdddab92-3e1f-49dc-afd0-8c84d0d952c2 | Oracle ID: a1ef35fe-46a7-4456-b194-24779b8fde68
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENTRAILS_FEASTER,
    oracle_id = "a1ef35fe-46a7-4456-b194-24779b8fde68",
    scryfall_id = "cdddab92-3e1f-49dc-afd0-8c84d0d952c2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Entrails Feaster",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::CAT],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
