//! Cabaretti Courtyard — (no cost) — Land
//! Oracle: When this land enters, sacrifice it. When you do, search your library for a basic Mountain, Forest, or Plains card, put it onto the battlefield tapped, then shuffle and you gain 1 life.
//! Set: EOC #151 — Edge of Eternities Commander | Scryfall ID: c54ddd4e-f668-4ec8-b123-59afa977eba4 | Oracle ID: 65424bea-fd53-4f85-9757-0b91a6d40ba4
// PARTIAL — the enter trigger sacrifices this land, fetches a basic
// Mountain/Forest/Plains onto the battlefield tapped (then shuffles) and gains
// 1 life; the printed reflexive "When you do" is the half the DSL cannot say.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card!(
    index = index::CABARETTI_COURTYARD,
    oracle_id = "65424bea-fd53-4f85-9757-0b91a6d40ba4",
    scryfall_id = "c54ddd4e-f668-4ec8-b123-59afa977eba4",
    faces = &[face!(name = "Cabaretti Courtyard", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the reflexive \"When you do\" trigger (CR 603.12) is not expressible: \
         the search and the 1 life are ordinary effects of the enter trigger, \
         so they still run when the sacrifice has become impossible"
    ),
    abilities = &[
        // NOT SUPPORTED: "When you do, search your library for a basic
        // Mountain, Forest, or Plains card, put it onto the battlefield tapped,
        // then shuffle and you gain 1 life." — a reflexive triggered ability
        // (CR 603.12), and no Trigger or Effect variant says one; the search
        // and the life gain therefore sit in the enter trigger's own effect
        // list and run even when the land was destroyed or bounced in response
        // and so was never sacrificed.
        triggered!(
            Trigger::ETB,
            &[
                Effect::SacrificeSelf,
                Effect::SearchLibrary {
                    filter: &Filter::And(&[
                        Filter::BASIC_LAND,
                        Filter::Or(&[
                            Filter::HasSubtype(land::MOUNTAIN),
                            Filter::HasSubtype(land::FOREST),
                            Filter::HasSubtype(land::PLAINS),
                        ]),
                    ]),
                    finds: &[Find::BATTLEFIELD_TAPPED],
                    optional: false,
                },
                Effect::gain_life(1),
            ]
        ),
    ],
);
