//! Belbe's Portal — {5} — Artifact
//! Oracle: As this artifact enters, choose a creature type.
//! Oracle: {3}, {T}: You may put a creature card of the chosen type from your hand onto the battlefield.
//! Set: NEM #127 — Nemesis | Scryfall ID: fb4eeea1-693e-475c-a209-8a0464df8081 | Oracle ID: 46e50ee9-91f8-416e-82fa-f1a76e61fb73
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BELBE_S_PORTAL,
    oracle_id = "46e50ee9-91f8-416e-82fa-f1a76e61fb73",
    scryfall_id = "fb4eeea1-693e-475c-a209-8a0464df8081",
    faces = &[face!(
        name = "Belbe's Portal",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
