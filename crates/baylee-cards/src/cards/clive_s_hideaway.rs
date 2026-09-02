//! Clive's Hideaway — (no cost) — Land — Town
//! Oracle: Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.)
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: You may play the exiled card without paying its mana cost if you control four or more legendary creatures.
//! Set: FIN #275 — Final Fantasy | Scryfall ID: 5e43c36f-b8a2-4b2b-b2ea-57e6fa97521c | Oracle ID: 283f743f-6e79-49de-b7ed-08e6ffb64cc6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 359,
    oracle_id: "283f743f-6e79-49de-b7ed-08e6ffb64cc6",
    scryfall_id: "5e43c36f-b8a2-4b2b-b2ea-57e6fa97521c",
    faces: &[
    face! {
        name: "Clive's Hideaway",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::TOWN],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
