//! Ancestor's Prophet — {4}{W} — Creature — Human Cleric
//! Oracle: Tap five untapped Clerics you control: You gain 10 life.
//! Set: ONS #3 — Onslaught | Scryfall ID: cdee956e-76b1-4ba7-a387-2fbfb853507d | Oracle ID: 274ce6b0-af29-4afc-873b-b4126a67bd81
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANCESTOR_S_PROPHET,
    oracle_id = "274ce6b0-af29-4afc-873b-b4126a67bd81",
    scryfall_id = "cdee956e-76b1-4ba7-a387-2fbfb853507d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Ancestor's Prophet",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
