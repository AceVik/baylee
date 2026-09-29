//! Throne of Bone — {1} — Artifact
//! Oracle: Whenever a player casts a black spell, you may pay {1}. If you do, you gain 1 life.
//! Set: 8ED #317 — Eighth Edition | Scryfall ID: 66ef3879-f708-4e2d-a1bc-6a75584fd8b1 | Oracle ID: f73c7edf-ed2c-41e8-ac83-c83ddd543f14
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THRONE_OF_BONE,
    oracle_id = "f73c7edf-ed2c-41e8-ac83-c83ddd543f14",
    scryfall_id = "66ef3879-f708-4e2d-a1bc-6a75584fd8b1",
    faces = &[face!(
        name = "Throne of Bone",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
