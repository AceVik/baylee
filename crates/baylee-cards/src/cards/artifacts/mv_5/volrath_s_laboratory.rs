//! Volrath's Laboratory — {5} — Artifact
//! Oracle: As this artifact enters, choose a color and a creature type.
//! Oracle: {5}, {T}: Create a 2/2 creature token of the chosen color and type.
//! Set: TPR #234 — Tempest Remastered | Scryfall ID: aa9b2bbd-0458-4ee8-8992-843169c2b137 | Oracle ID: f2a6e26c-6bd6-4fca-ba9f-d52d01fc03ef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VOLRATH_S_LABORATORY,
    oracle_id = "f2a6e26c-6bd6-4fca-ba9f-d52d01fc03ef",
    scryfall_id = "aa9b2bbd-0458-4ee8-8992-843169c2b137",
    faces = &[face!(
        name = "Volrath's Laboratory",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
