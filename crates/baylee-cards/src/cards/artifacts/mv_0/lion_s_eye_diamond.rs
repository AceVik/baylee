//! Lion's Eye Diamond — {0} — Artifact
//! Oracle: Discard your hand, Sacrifice this artifact: Add three mana of any one color. Activate only as an instant.
//! Set: VMA #271 — Vintage Masters | Scryfall ID: 758f95f8-bcb0-43ae-b474-56ebd855951e | Oracle ID: ee6099b0-fb1f-42f1-b862-7708c6e36d05
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LION_S_EYE_DIAMOND,
    oracle_id = "ee6099b0-fb1f-42f1-b862-7708c6e36d05",
    scryfall_id = "758f95f8-bcb0-43ae-b474-56ebd855951e",
    faces = &[face!(
        name = "Lion's Eye Diamond",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
