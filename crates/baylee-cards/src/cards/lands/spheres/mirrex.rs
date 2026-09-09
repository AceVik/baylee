//! Mirrex — (no cost) — Land — Sphere
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Activate only if this land entered this turn.
//! Oracle: {3}, {T}: Create a 1/1 colorless Phyrexian Mite artifact creature token with toxic 1 and "This token can't block." (Players dealt combat damage by it also get a poison counter.)
//! Set: ONE #254 — Phyrexia: All Will Be One | Scryfall ID: 54a702cd-ca49-4570-b47e-8b090452a3c3 | Oracle ID: 5502741a-e3b9-454e-8121-4360a6db6750
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 764,
    oracle_id: "5502741a-e3b9-454e-8121-4360a6db6750",
    scryfall_id: "54a702cd-ca49-4570-b47e-8b090452a3c3",
    faces: &[
    face! {
        name: "Mirrex",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::SPHERE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
