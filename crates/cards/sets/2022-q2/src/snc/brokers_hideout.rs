//! Brokers Hideout — (no cost) — Land
//! Oracle: When this land enters, sacrifice it. When you do, search your library for a basic Forest, Plains, or Island card, put it onto the battlefield tapped, then shuffle and you gain 1 life.
//! Set: SNC #248 — Streets of New Capenna | Scryfall ID: 989b299b-daa9-4bda-94e2-9a2f0e8f2bce | Oracle ID: bd002797-a545-4bee-88bf-b878436e7cca
// IMPLEMENTED — the enter trigger sacrifices the land, and the printed "When
// you do" is a reflexive triggered ability (CR 603.12) written after it: it
// exists only if the sacrifice happened, goes on the stack of its own, and
// searches for a basic Forest, Plains or Island onto the battlefield tapped
// (the shuffle after a search is derived) and gains 1 life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

// "a basic Forest, Plains or Island card": the Basic supertype (CR 205.4a) and
// one of the three printed land types, in the order the sentence names them.
static FINDS: Filter = Filter::And(&[
    Filter::BASIC_LAND,
    Filter::Or(&[
        Filter::HasSubtype(land::FOREST),
        Filter::HasSubtype(land::PLAINS),
        Filter::HasSubtype(land::ISLAND),
    ]),
]);

card!(
    index = index::BROKERS_HIDEOUT,
    oracle_id = "bd002797-a545-4bee-88bf-b878436e7cca",
    scryfall_id = "989b299b-daa9-4bda-94e2-9a2f0e8f2bce",
    faces = &[face!(name = "Brokers Hideout", types = TypeSet::LAND,),],
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
