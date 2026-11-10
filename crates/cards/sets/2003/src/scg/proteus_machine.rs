//! Proteus Machine — {3} — Artifact Creature — Shapeshifter
//! Oracle: Morph {0} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, it becomes the creature type of your choice. (This effect lasts indefinitely.)
//! Set: SCG #141 — Scourge | Scryfall ID: d1c8cff1-b289-41a4-9fa3-cc5e7ba70802 | Oracle ID: c3acf94e-3adc-4397-a736-3f6072a5967d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PROTEUS_MACHINE,
    oracle_id = "c3acf94e-3adc-4397-a736-3f6072a5967d",
    scryfall_id = "d1c8cff1-b289-41a4-9fa3-cc5e7ba70802",
    faces = &[face!(
        name = "Proteus Machine",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
