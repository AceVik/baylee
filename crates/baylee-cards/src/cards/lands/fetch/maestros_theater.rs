//! Maestros Theater — (no cost) — Land
//! Oracle: When this land enters, sacrifice it. When you do, search your library for a basic Island, Swamp, or Mountain card, put it onto the battlefield tapped, then shuffle and you gain 1 life.
//! Set: EOC #167 — Edge of Eternities Commander | Scryfall ID: d4087ba6-8227-4ec3-a989-0833ad6c8788 | Oracle ID: 9464ddf2-4bcb-44f6-b945-89a132544de6
// IMPLEMENTED — ETB: sacrifice this land, then search up a basic Island,
// Swamp or Mountain onto the battlefield tapped and gain 1 life. The printed
// "When you do" is a mandatory sacrifice in the same resolution, so the
// reflexive half is the rest of one effect list; the shuffle afterwards is
// derived, not declared (CR 701.19).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card!(
    index = index::MAESTROS_THEATER,
    oracle_id = "9464ddf2-4bcb-44f6-b945-89a132544de6",
    scryfall_id = "d4087ba6-8227-4ec3-a989-0833ad6c8788",
    faces = &[face!(name = "Maestros Theater", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::ETB,
        &[
            Effect::SacrificeSelf,
            Effect::SearchLibrary {
                filter: &Filter::And(&[
                    Filter::BASIC_LAND,
                    Filter::Or(&[
                        Filter::HasSubtype(land::ISLAND),
                        Filter::HasSubtype(land::SWAMP),
                        Filter::HasSubtype(land::MOUNTAIN),
                    ]),
                ]),
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: false,
            },
            Effect::gain_life(1),
        ]
    )],
);
