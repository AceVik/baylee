//! Aphetto Runecaster — {3}{U} — Creature — Human Wizard
//! Oracle: Whenever a permanent is turned face up, you may draw a card.
//! Set: SCG #28 — Scourge | Scryfall ID: 7c5de028-91ce-48d8-8557-ae12542adea2 | Oracle ID: 949eb86a-94b2-4c9f-8040-f4ad8e1a7e33
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::APHETTO_RUNECASTER,
    oracle_id = "949eb86a-94b2-4c9f-8040-f4ad8e1a7e33",
    scryfall_id = "7c5de028-91ce-48d8-8557-ae12542adea2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Aphetto Runecaster",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
