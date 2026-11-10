//! Storage Matrix — {3} — Artifact
//! Oracle: As long as this artifact is untapped, each player chooses artifact, creature, or land during their untap step. That player can untap only permanents of the chosen type this step.
//! Set: 9ED #310 — Ninth Edition | Scryfall ID: 2aa41221-ee75-4713-99bd-683013ba64de | Oracle ID: 03136fe1-17e2-4b1c-860a-dc6373c8fdb5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STORAGE_MATRIX,
    oracle_id = "03136fe1-17e2-4b1c-860a-dc6373c8fdb5",
    scryfall_id = "2aa41221-ee75-4713-99bd-683013ba64de",
    faces = &[face!(
        name = "Storage Matrix",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
