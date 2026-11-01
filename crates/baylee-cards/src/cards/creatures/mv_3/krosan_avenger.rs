//! Krosan Avenger — {2}{G} — Creature — Human Druid
//! Oracle: Trample
//! Oracle: Threshold — {1}{G}: Regenerate this creature. Activate only if there are seven or more cards in your graveyard.
//! Set: ODY #247 — Odyssey | Scryfall ID: 0afd6911-32b5-410a-afb0-fd3d2996fe59 | Oracle ID: 8991b367-6fc1-4de7-a1ee-4edb57c44fc2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KROSAN_AVENGER,
    oracle_id = "8991b367-6fc1-4de7-a1ee-4edb57c44fc2",
    scryfall_id = "0afd6911-32b5-410a-afb0-fd3d2996fe59",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Krosan Avenger",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(3),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
