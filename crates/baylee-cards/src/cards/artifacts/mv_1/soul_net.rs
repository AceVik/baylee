//! Soul Net — {1} — Artifact
//! Oracle: Whenever a creature dies, you may pay {1}. If you do, you gain 1 life.
//! Set: 7ED #317 — Seventh Edition | Scryfall ID: 62e267df-22e1-422e-973b-d192b40d5f19 | Oracle ID: 6021c2d6-d098-4de2-9c7e-4c571f9238f6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOUL_NET,
    oracle_id = "6021c2d6-d098-4de2-9c7e-4c571f9238f6",
    scryfall_id = "62e267df-22e1-422e-973b-d192b40d5f19",
    faces = &[face!(
        name = "Soul Net",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
