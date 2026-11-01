//! Phyrexian Portal — {3} — Artifact
//! Oracle: {3}: If your library has ten or more cards in it, target opponent looks at the top ten cards of your library and separates them into two face-down piles. Exile one of those piles. Search the other pile for a card, put it into your hand, then shuffle the rest of that pile into your library.
//! Set: ME2 #217 — Masters Edition II | Scryfall ID: c165b61b-3cd5-4035-a9d3-1c03289ae5ba | Oracle ID: 1a3ce329-ee48-4abc-b820-b43e9e68eae9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PHYREXIAN_PORTAL,
    oracle_id = "1a3ce329-ee48-4abc-b820-b43e9e68eae9",
    scryfall_id = "c165b61b-3cd5-4035-a9d3-1c03289ae5ba",
    faces = &[face!(
        name = "Phyrexian Portal",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
