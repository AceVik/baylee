//! Matopi Golem — {5} — Artifact Creature — Golem
//! Oracle: {1}: Regenerate this creature. When it regenerates this way, put a -1/-1 counter on it.
//! Set: VIS #150 — Visions | Scryfall ID: 92378d6f-89ee-49dc-8964-0e9c55daeffc | Oracle ID: 0308b9f2-11da-41d0-8d18-18e8c0c600ab
// PARTIAL: a 3/3 body only; the {1} regeneration is left out, because it cannot be given its "when it regenerates this way, put a -1/-1 counter on it" cost, and regeneration without that counter would be stronger than printed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MATOPI_GOLEM,
    oracle_id = "0308b9f2-11da-41d0-8d18-18e8c0c600ab",
    scryfall_id = "92378d6f-89ee-49dc-8964-0e9c55daeffc",
    coverage = Coverage::Partial(
        "a 3/3 body only; the {1} regeneration is left out, because it cannot be given its \"when it regenerates this way, put a -1/-1 counter on it\" cost, and regeneration without that counter would be stronger than printed"
    ),
    faces = &[face!(
        name = "Matopi Golem",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(3),
        toughness = Some(3),
    ),],
    // NOT SUPPORTED: "{1}: Regenerate this creature. When it regenerates this way, put a -1/-1
    // counter on it." A regeneration shield carries no rider that fires when it is used, so the
    // counter cannot follow the regeneration; the old ability regenerated for {1} for ever.
);
