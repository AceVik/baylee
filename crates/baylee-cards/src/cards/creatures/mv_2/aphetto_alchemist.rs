//! Aphetto Alchemist — {1}{U} — Creature — Human Wizard
//! Oracle: {T}: Untap target artifact or creature.
//! Oracle: Morph {U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #64 — Onslaught | Scryfall ID: dfd2628f-63c4-4e19-83ea-26041650faab | Oracle ID: 867aa0b7-b814-4d33-a677-f6368c8e93c0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::APHETTO_ALCHEMIST,
    oracle_id = "867aa0b7-b814-4d33-a677-f6368c8e93c0",
    scryfall_id = "dfd2628f-63c4-4e19-83ea-26041650faab",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Aphetto Alchemist",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
