//! Cephalid Vandal — {1}{U} — Creature — Octopus Rogue
//! Oracle: At the beginning of your upkeep, put a shred counter on this creature. Then mill a card for each shred counter on this creature.
//! Set: TOR #31 — Torment | Scryfall ID: 2500976f-a329-4a51-bcc1-cafe39bc3ccf | Oracle ID: f48dae33-ebf9-41ee-a61e-7573a2733db2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CEPHALID_VANDAL,
    oracle_id = "f48dae33-ebf9-41ee-a61e-7573a2733db2",
    scryfall_id = "2500976f-a329-4a51-bcc1-cafe39bc3ccf",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Cephalid Vandal",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OCTOPUS, subtypes::creature::ROGUE],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
