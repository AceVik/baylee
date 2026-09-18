//! Ramunap Excavator — {2}{G} — Creature — Snake Cleric
//! Oracle: You may play lands from your graveyard.
//! Set: OTC #202 — Outlaws of Thunder Junction Commander | Scryfall ID: 3f8a0a0e-81f7-40a2-b393-bdc1423549f6 | Oracle ID: 4f819ba4-52ef-4fdd-8e4c-5ae3b2f44db5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAMUNAP_EXCAVATOR,
    oracle_id = "4f819ba4-52ef-4fdd-8e4c-5ae3b2f44db5",
    scryfall_id = "3f8a0a0e-81f7-40a2-b393-bdc1423549f6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Ramunap Excavator",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
