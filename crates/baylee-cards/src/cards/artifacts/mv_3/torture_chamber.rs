//! Torture Chamber — {3} — Artifact
//! Oracle: At the beginning of your upkeep, put a pain counter on this artifact.
//! Oracle: At the beginning of your end step, this artifact deals damage to you equal to the number of pain counters on it.
//! Oracle: {1}, {T}, Remove all pain counters from this artifact: It deals damage to target creature equal to the number of pain counters removed this way.
//! Set: TMP #313 — Tempest | Scryfall ID: 5648158d-38d4-4167-8af5-ee5d7d6fd7cb | Oracle ID: 12436f2f-eaed-4b46-8570-81316ce15962
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TORTURE_CHAMBER,
    oracle_id = "12436f2f-eaed-4b46-8570-81316ce15962",
    scryfall_id = "5648158d-38d4-4167-8af5-ee5d7d6fd7cb",
    faces = &[face!(
        name = "Torture Chamber",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
