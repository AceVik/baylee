//! Matopi Golem — {5} — Artifact Creature — Golem
//! Oracle: {1}: Regenerate this creature. When it regenerates this way, put a -1/-1 counter on it.
//! Set: VIS #150 — Visions | Scryfall ID: 92378d6f-89ee-49dc-8964-0e9c55daeffc | Oracle ID: 0308b9f2-11da-41d0-8d18-18e8c0c600ab
// PARTIAL: {1} regenerates Matopi Golem; putting a -1/-1 counter on it when it regenerates this way is not yet supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: When it regenerates this way, put a -1/-1 counter on it.
card!(
    index = index::MATOPI_GOLEM,
    oracle_id = "0308b9f2-11da-41d0-8d18-18e8c0c600ab",
    scryfall_id = "92378d6f-89ee-49dc-8964-0e9c55daeffc",
    coverage = Coverage::Partial(
        "{1} regenerates Matopi Golem; putting a -1/-1 counter on it when it regenerates this way is not yet supported"
    ),
    faces = &[face!(
        name = "Matopi Golem",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{1}"),
        &[Effect::regenerate(TargetSpec::ThisObject)],
    )],
);
