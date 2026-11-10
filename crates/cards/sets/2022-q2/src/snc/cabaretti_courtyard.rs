//! Cabaretti Courtyard — (no cost) — Land
//! Oracle: When this land enters, sacrifice it. When you do, search your library for a basic Mountain, Forest, or Plains card, put it onto the battlefield tapped, then shuffle and you gain 1 life.
//! Set: EOC #151 — Edge of Eternities Commander | Scryfall ID: c54ddd4e-f668-4ec8-b123-59afa977eba4 | Oracle ID: 65424bea-fd53-4f85-9757-0b91a6d40ba4
// IMPLEMENTED — the enter trigger sacrifices the land, and the printed "When
// you do" is a reflexive triggered ability (CR 603.12) written after it: it
// exists only if the sacrifice happened, goes on the stack of its own, and
// searches for a basic Mountain, Forest or Plains onto the battlefield tapped
// (the shuffle after a search is derived) and gains 1 life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

// "a basic Mountain, Forest or Plains card": the Basic supertype (CR 205.4a)
// and one of the three printed land types, in the order the sentence names
// them.
static FINDS: Filter = Filter::And(&[
    Filter::BASIC_LAND,
    Filter::Or(&[
        Filter::HasSubtype(land::MOUNTAIN),
        Filter::HasSubtype(land::FOREST),
        Filter::HasSubtype(land::PLAINS),
    ]),
]);

card!(
    index = index::CABARETTI_COURTYARD,
    oracle_id = "65424bea-fd53-4f85-9757-0b91a6d40ba4",
    scryfall_id = "c54ddd4e-f668-4ec8-b123-59afa977eba4",
    faces = &[face!(name = "Cabaretti Courtyard", types = TypeSet::LAND,),],
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
