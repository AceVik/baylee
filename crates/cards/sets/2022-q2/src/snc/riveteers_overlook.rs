//! Riveteers Overlook — (no cost) — Land
//! Oracle: When this land enters, sacrifice it. When you do, search your library for a basic Swamp, Mountain, or Forest card, put it onto the battlefield tapped, then shuffle and you gain 1 life.
//! Set: ECC #162 — Lorwyn Eclipsed Commander | Scryfall ID: 65ce9590-87c0-4057-bddb-fadc0de552f6 | Oracle ID: 5548ff43-e5f6-4a63-8562-a2b1de06d6f5
// IMPLEMENTED — the enter trigger sacrifices the land, and the printed "When
// you do" is a reflexive triggered ability (CR 603.12) written after it: it
// exists only if the sacrifice happened, goes on the stack of its own, and
// searches for a basic Swamp, Mountain or Forest onto the battlefield tapped
// (the shuffle after a search is derived) and gains 1 life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

// "a basic Swamp, Mountain or Forest card": the Basic supertype (CR 205.4a)
// and one of the three printed land types, in the order the sentence names
// them.
static FINDS: Filter = Filter::And(&[
    Filter::BASIC_LAND,
    Filter::Or(&[
        Filter::HasSubtype(land::SWAMP),
        Filter::HasSubtype(land::MOUNTAIN),
        Filter::HasSubtype(land::FOREST),
    ]),
]);

card!(
    index = index::RIVETEERS_OVERLOOK,
    oracle_id = "5548ff43-e5f6-4a63-8562-a2b1de06d6f5",
    scryfall_id = "65ce9590-87c0-4057-bddb-fadc0de552f6",
    faces = &[face!(name = "Riveteers Overlook", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::ETB,
        &[
            Effect::SacrificeSelf,
            Effect::Reflexive {
                when: ReflexiveEvent::SacrificedThis,
                effects: &[
                    Effect::SearchLibrary {
                        filter: &FINDS,
                        finds: &[Find::BATTLEFIELD_TAPPED],
                        optional: false,
                    },
                    Effect::gain_life(1),
                ],
                target: None,
            },
        ]
    )],
);
