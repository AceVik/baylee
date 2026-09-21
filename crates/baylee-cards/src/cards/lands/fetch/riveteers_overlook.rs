//! Riveteers Overlook — (no cost) — Land
//! Oracle: When this land enters, sacrifice it. When you do, search your library for a basic Swamp, Mountain, or Forest card, put it onto the battlefield tapped, then shuffle and you gain 1 life.
//! Set: ECC #162 — Lorwyn Eclipsed Commander | Scryfall ID: 65ce9590-87c0-4057-bddb-fadc0de552f6 | Oracle ID: 5548ff43-e5f6-4a63-8562-a2b1de06d6f5
// PARTIAL — the entering trigger sacrifices the land, then searches for a basic
// Swamp, Mountain or Forest card onto the battlefield tapped and gains 1 life.
// The second half is printed as a reflexive trigger ("When you do"), which the
// DSL cannot state as its own trigger; see the note on the ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card!(
    index = index::RIVETEERS_OVERLOOK,
    oracle_id = "5548ff43-e5f6-4a63-8562-a2b1de06d6f5",
    scryfall_id = "65ce9590-87c0-4057-bddb-fadc0de552f6",
    faces = &[face!(name = "Riveteers Overlook", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "\"When you do\" is a reflexive trigger (CR 603.12) and Trigger has no event \
         for it, so the search and the life are the entering trigger's next effects \
         and still happen if the land is answered before that trigger resolves",
    ),
    abilities = &[
        // NOT SUPPORTED: "When you do" — a reflexive trigger with no DSL event.
        triggered!(
            Trigger::ETB,
            &[
                Effect::SacrificeSelf,
                Effect::SearchLibrary {
                    filter: &Filter::And(&[
                        Filter::HasSupertype(SupertypeSet::BASIC),
                        Filter::Or(&[
                            Filter::HasSubtype(land::SWAMP),
                            Filter::HasSubtype(land::MOUNTAIN),
                            Filter::HasSubtype(land::FOREST),
                        ]),
                    ]),
                    finds: &[Find::BATTLEFIELD_TAPPED],
                    optional: false,
                },
                Effect::gain_life(1),
            ],
        ),
    ],
);
