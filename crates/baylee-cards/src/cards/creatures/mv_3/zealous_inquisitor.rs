//! Zealous Inquisitor — {2}{W} — Creature — Human Cleric
//! Oracle: {1}{W}: The next 1 damage that would be dealt to this creature this turn is dealt to target creature instead.
//! Set: 9ED #57 — Ninth Edition | Scryfall ID: 7621b1af-fdbd-4604-b37c-04fb8f68c3b6 | Oracle ID: e85f6251-8801-414b-adc9-5488794e7456
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZEALOUS_INQUISITOR,
    oracle_id = "e85f6251-8801-414b-adc9-5488794e7456",
    scryfall_id = "7621b1af-fdbd-4604-b37c-04fb8f68c3b6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Zealous Inquisitor",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
