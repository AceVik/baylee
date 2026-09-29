//! Iron Star — {1} — Artifact
//! Oracle: Whenever a player casts a red spell, you may pay {1}. If you do, you gain 1 life.
//! Set: 8ED #304 — Eighth Edition | Scryfall ID: ec6d1e6b-80af-4eb1-be3c-62ab74315777 | Oracle ID: e9ec67e1-7064-44d4-a1ed-04b7893ffb15
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IRON_STAR,
    oracle_id = "e9ec67e1-7064-44d4-a1ed-04b7893ffb15",
    scryfall_id = "ec6d1e6b-80af-4eb1-be3c-62ab74315777",
    faces = &[face!(
        name = "Iron Star",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
