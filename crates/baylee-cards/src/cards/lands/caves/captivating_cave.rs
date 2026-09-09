//! Captivating Cave — (no cost) — Land — Cave
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {4}, {T}, Sacrifice this land: Put two +1/+1 counters on target creature. Activate only as a sorcery.
//! Set: LCI #268 — The Lost Caverns of Ixalan | Scryfall ID: 1d1a645e-85c7-4044-b817-6e24744d245e | Oracle ID: 4c77767a-8133-43bc-b7a5-09a73259d354
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 329,
    oracle_id: "4c77767a-8133-43bc-b7a5-09a73259d354",
    scryfall_id: "1d1a645e-85c7-4044-b817-6e24744d245e",
    faces: &[
    face! {
        name: "Captivating Cave",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
