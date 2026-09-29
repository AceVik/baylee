//! Wooden Sphere — {1} — Artifact
//! Oracle: Whenever a player casts a green spell, you may pay {1}. If you do, you gain 1 life.
//! Set: 8ED #321 — Eighth Edition | Scryfall ID: 5310b3a7-0a85-4469-be88-5c7ab1de4f4a | Oracle ID: 0bd8917c-bec4-4603-bc3f-8e0c2afae56a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WOODEN_SPHERE,
    oracle_id = "0bd8917c-bec4-4603-bc3f-8e0c2afae56a",
    scryfall_id = "5310b3a7-0a85-4469-be88-5c7ab1de4f4a",
    faces = &[face!(
        name = "Wooden Sphere",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
