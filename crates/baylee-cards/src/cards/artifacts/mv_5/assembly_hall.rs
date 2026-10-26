//! Assembly Hall — {5} — Artifact
//! Oracle: {4}, {T}: Reveal a creature card in your hand. Search your library for a card with the same name as that card, reveal it, put it into your hand, then shuffle.
//! Set: MMQ #286 — Mercadian Masques | Scryfall ID: 1676ccbb-91d2-4f26-b3a5-ccb1a21bdebf | Oracle ID: 8da9f975-519d-455a-80c1-2b468cad79a1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASSEMBLY_HALL,
    oracle_id = "8da9f975-519d-455a-80c1-2b468cad79a1",
    scryfall_id = "1676ccbb-91d2-4f26-b3a5-ccb1a21bdebf",
    faces = &[face!(
        name = "Assembly Hall",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
