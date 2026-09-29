//! Sedge Troll — {2}{R} — Creature — Troll
//! Oracle: This creature gets +1/+1 as long as you control a Swamp.
//! Oracle: {B}: Regenerate this creature.
//! Set: ME4 #135 — Masters Edition IV | Scryfall ID: 78de6044-1ed8-4fd9-822c-69658e3f3fba | Oracle ID: a6a43190-cca2-4f07-afe9-8af681d777da
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SEDGE_TROLL,
    oracle_id = "a6a43190-cca2-4f07-afe9-8af681d777da",
    scryfall_id = "78de6044-1ed8-4fd9-822c-69658e3f3fba",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Sedge Troll",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TROLL],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
