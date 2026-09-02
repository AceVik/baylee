//! The Seedcore — (no cost) — Land — Sphere
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast Phyrexian creature spells.
//! Oracle: Corrupted — {T}: Target 1/1 creature gets +2/+1 until end of turn. Activate only if an opponent has three or more poison counters.
//! Set: ONE #259 — Phyrexia: All Will Be One | Scryfall ID: 29c91aad-bf33-448e-b122-65940fb2e33b | Oracle ID: 249fdd3e-376c-4ec2-a612-4353e0e61ee2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1179,
    oracle_id: "249fdd3e-376c-4ec2-a612-4353e0e61ee2",
    scryfall_id: "29c91aad-bf33-448e-b122-65940fb2e33b",
    faces: &[
    face! {
        name: "The Seedcore",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::SPHERE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
