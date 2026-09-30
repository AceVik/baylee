//! Jade Monolith — {4} — Artifact
//! Oracle: {1}: The next time a source of your choice would deal damage to target creature this turn, that source deals that damage to you instead.
//! Set: ME4 #208 — Masters Edition IV | Scryfall ID: 482d9aab-7ff1-47ee-9862-8a8c687f14bf | Oracle ID: 1e105ab7-fb10-4cfd-ac2f-5e11488cf1b0
// PARTIAL — redirecting a chosen source's damage from a creature to you is
// not in the engine.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JADE_MONOLITH,
    oracle_id = "1e105ab7-fb10-4cfd-ac2f-5e11488cf1b0",
    scryfall_id = "482d9aab-7ff1-47ee-9862-8a8c687f14bf",
    coverage = Coverage::Partial(
        "redirecting a chosen source's damage from a creature to you is not in the engine"
    ),
    faces = &[face!(
        name = "Jade Monolith",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    // NOT SUPPORTED: {1}: The next time a source of your choice would deal damage to
    // target creature this turn, that source deals that damage to you instead.
);
