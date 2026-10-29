//! Riptide Replicator — {X}{4} — Artifact
//! Oracle: As this artifact enters, choose a color and a creature type.
//! Oracle: This artifact enters with X charge counters on it.
//! Oracle: {4}, {T}: Create an X/X creature token of the chosen color and type, where X is the number of charge counters on this artifact.
//! Set: ONS #309 — Onslaught | Scryfall ID: 41bb314f-237a-43fc-95c8-b26188dc4476 | Oracle ID: f526ce8a-27f5-4785-aed3-944b080c9c1a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RIPTIDE_REPLICATOR,
    oracle_id = "f526ce8a-27f5-4785-aed3-944b080c9c1a",
    scryfall_id = "41bb314f-237a-43fc-95c8-b26188dc4476",
    faces = &[face!(
        name = "Riptide Replicator",
        mana_cost = mana!("{X}{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
