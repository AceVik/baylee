//! Hostile Desert — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, Exile a land card from your graveyard: This land becomes a 3/4 Elemental creature until end of turn. It's still a land.
//! Set: MKC #266 — Murders at Karlov Manor Commander | Scryfall ID: d71031c2-7379-4d83-b6d6-61f3104593c4 | Oracle ID: 41459587-7509-404e-bd7d-fb8831dee789
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 638,
    oracle_id: "41459587-7509-404e-bd7d-fb8831dee789",
    scryfall_id: "d71031c2-7379-4d83-b6d6-61f3104593c4",
    faces: &[
    face! {
        name: "Hostile Desert",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::DESERT],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
