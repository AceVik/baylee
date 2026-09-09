//! The Gold Saucer — (no cost) — Land — Town
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Flip a coin. If you win the flip, create a Treasure token.
//! Oracle: {3}, {T}, Sacrifice two artifacts: Draw a card.
//! Set: FIN #279 — Final Fantasy | Scryfall ID: 5363c881-443d-43df-afd8-f81e1a1741a2 | Oracle ID: 93e38650-ce22-4ab9-b79d-cc7b6477c075
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1172,
    oracle_id: "93e38650-ce22-4ab9-b79d-cc7b6477c075",
    scryfall_id: "5363c881-443d-43df-afd8-f81e1a1741a2",
    faces: &[
    face! {
        name: "The Gold Saucer",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::TOWN],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
