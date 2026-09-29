//! Gauntlet of Might — {4} — Artifact
//! Oracle: Red creatures get +1/+1.
//! Oracle: Whenever a Mountain is tapped for mana, its controller adds an additional {R}.
//! Set: ME4 #202 — Masters Edition IV | Scryfall ID: df3bc33f-23f8-4eb4-a70e-b8b7af5b40f6 | Oracle ID: d38ad188-515e-4865-a0ed-5d0fd4c7b453
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GAUNTLET_OF_MIGHT,
    oracle_id = "d38ad188-515e-4865-a0ed-5d0fd4c7b453",
    scryfall_id = "df3bc33f-23f8-4eb4-a70e-b8b7af5b40f6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Gauntlet of Might",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
