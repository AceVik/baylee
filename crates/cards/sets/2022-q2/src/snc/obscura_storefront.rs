//! Obscura Storefront — (no cost) — Land
//! Oracle: When this land enters, sacrifice it. When you do, search your library for a basic Plains, Island, or Swamp card, put it onto the battlefield tapped, then shuffle and you gain 1 life.
//! Set: DSC #291 — Duskmourn: House of Horror Commander | Scryfall ID: d8eaf8d2-8029-49d9-a94b-a72dc31fc81f | Oracle ID: dc31a6f8-6228-4a25-b937-5d8d78514333
// IMPLEMENTED — the enter trigger sacrifices the land, and the printed "When
// you do" is a reflexive triggered ability (CR 603.12) written after it: it
// exists only if the sacrifice happened, goes on the stack of its own, and
// searches for a basic Plains, Island or Swamp onto the battlefield tapped
// (the shuffle after a search is derived) and gains 1 life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

// "a basic Plains, Island or Swamp card": the Basic supertype (CR 205.4a) and
// one of the three printed land types, in the order the sentence names them.
static FINDS: Filter = Filter::And(&[
    Filter::BASIC_LAND,
    Filter::Or(&[
        Filter::HasSubtype(land::PLAINS),
        Filter::HasSubtype(land::ISLAND),
        Filter::HasSubtype(land::SWAMP),
    ]),
]);

card!(
    index = index::OBSCURA_STOREFRONT,
    oracle_id = "dc31a6f8-6228-4a25-b937-5d8d78514333",
    scryfall_id = "d8eaf8d2-8029-49d9-a94b-a72dc31fc81f",
    faces = &[face!(name = "Obscura Storefront", types = TypeSet::LAND,),],
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
