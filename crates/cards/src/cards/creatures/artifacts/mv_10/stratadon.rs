//! Stratadon — {10} — Artifact Creature — Beast
//! Oracle: Domain — This spell costs {1} less to cast for each basic land type among lands you control.
//! Oracle: Trample
//! Set: PLS #135 — Planeshift | Scryfall ID: 324bc757-9942-4862-b691-5af42e07f682 | Oracle ID: 3b78b672-7773-44c6-9a21-f89c058966ff
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STRATADON,
    oracle_id = "3b78b672-7773-44c6-9a21-f89c058966ff",
    scryfall_id = "324bc757-9942-4862-b691-5af42e07f682",
    faces = &[face!(
        name = "Stratadon",
        mana_cost = mana!("{10}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::BEAST],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
