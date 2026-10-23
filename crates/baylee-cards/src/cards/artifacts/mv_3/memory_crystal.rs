//! Memory Crystal — {3} — Artifact
//! Oracle: Buyback costs cost {2} less.
//! Set: EXO #134 — Exodus | Scryfall ID: c786ea5b-52ad-4d1b-855e-ce6d0b9af67e | Oracle ID: f5fd3141-6fd9-4491-9f29-7fd81209bd57

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MEMORY_CRYSTAL,
    oracle_id = "f5fd3141-6fd9-4491-9f29-7fd81209bd57",
    scryfall_id = "c786ea5b-52ad-4d1b-855e-ce6d0b9af67e",
    coverage = Coverage::Partial(
        "Buyback and reducing buyback costs are not supported; this card enters the battlefield as an inert artifact."
    ),
    faces = &[face!(
        name = "Memory Crystal",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
);
