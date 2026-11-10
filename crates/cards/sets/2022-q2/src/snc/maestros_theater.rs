//! Maestros Theater — (no cost) — Land
//! Oracle: When this land enters, sacrifice it. When you do, search your library for a basic Island, Swamp, or Mountain card, put it onto the battlefield tapped, then shuffle and you gain 1 life.
//! Set: EOC #167 — Edge of Eternities Commander | Scryfall ID: d4087ba6-8227-4ec3-a989-0833ad6c8788 | Oracle ID: 9464ddf2-4bcb-44f6-b945-89a132544de6
// IMPLEMENTED — the enter trigger sacrifices the land, and the printed "When
// you do" is a reflexive triggered ability (CR 603.12) written after it: it
// exists only if the sacrifice happened, goes on the stack of its own, and
// searches for a basic Island, Swamp or Mountain onto the battlefield tapped
// (the shuffle after a search is derived) and gains 1 life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

// "a basic Island, Swamp or Mountain card": the Basic supertype (CR 205.4a)
// and one of the three printed land types, in the order the sentence names
// them.
static FINDS: Filter = Filter::And(&[
    Filter::BASIC_LAND,
    Filter::Or(&[
        Filter::HasSubtype(land::ISLAND),
        Filter::HasSubtype(land::SWAMP),
        Filter::HasSubtype(land::MOUNTAIN),
    ]),
]);

card!(
    index = index::MAESTROS_THEATER,
    oracle_id = "9464ddf2-4bcb-44f6-b945-89a132544de6",
    scryfall_id = "d4087ba6-8227-4ec3-a989-0833ad6c8788",
    faces = &[face!(name = "Maestros Theater", types = TypeSet::LAND,),],
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
