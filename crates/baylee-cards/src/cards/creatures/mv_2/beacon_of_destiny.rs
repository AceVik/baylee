//! Beacon of Destiny — {1}{W} — Creature — Human Cleric
//! Oracle: {T}: The next time a source of your choice would deal damage to you this turn, that damage is dealt to this creature instead.
//! Set: LGN #5 — Legions | Scryfall ID: 30b1cad7-4e96-4ebe-8c99-4ed9217becf3 | Oracle ID: a1ed2274-7774-4c65-a95b-28ae5e225994
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BEACON_OF_DESTINY,
    oracle_id = "a1ed2274-7774-4c65-a95b-28ae5e225994",
    scryfall_id = "30b1cad7-4e96-4ebe-8c99-4ed9217becf3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Beacon of Destiny",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
