//! Emblazoned Golem — {2} — Artifact Creature — Golem
//! Oracle: Kicker {X} (You may pay an additional {X} as you cast this spell.)
//! Oracle: Spend only colored mana on X. No more than one mana of each color may be spent this way.
//! Oracle: If this creature was kicked, it enters with X +1/+1 counters on it.
//! Set: APC #136 — Apocalypse | Scryfall ID: 98527fc6-4f4c-4ded-9e72-49186b7e5bd3 | Oracle ID: fbeafc81-1ddb-4b54-8f86-e6afd5f81db8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMBLAZONED_GOLEM,
    oracle_id = "fbeafc81-1ddb-4b54-8f86-e6afd5f81db8",
    scryfall_id = "98527fc6-4f4c-4ded-9e72-49186b7e5bd3",
    faces = &[face!(
        name = "Emblazoned Golem",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
