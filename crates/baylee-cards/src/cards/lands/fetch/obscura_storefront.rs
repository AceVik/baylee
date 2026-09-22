//! Obscura Storefront — (no cost) — Land
//! Oracle: When this land enters, sacrifice it. When you do, search your library for a basic Plains, Island, or Swamp card, put it onto the battlefield tapped, then shuffle and you gain 1 life.
//! Set: DSC #291 — Duskmourn: House of Horror Commander | Scryfall ID: d8eaf8d2-8029-49d9-a94b-a72dc31fc81f | Oracle ID: dc31a6f8-6228-4a25-b937-5d8d78514333
// PARTIAL — the enter trigger builds in full: the land sacrifices itself,
// fetches a basic Plains, Island or Swamp onto the battlefield tapped and you
// gain 1 life. What the DSL cannot say is the printed "When you do", a
// reflexive trigger (CR 603.12), so the fetch rides on the sacrifice's own
// resolution instead of on a second trigger that exists only because the
// sacrifice happened.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card!(
    index = index::OBSCURA_STOREFRONT,
    oracle_id = "dc31a6f8-6228-4a25-b937-5d8d78514333",
    scryfall_id = "d8eaf8d2-8029-49d9-a94b-a72dc31fc81f",
    faces = &[face!(name = "Obscura Storefront", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "\"When you do\" is a reflexive trigger (CR 603.12) and the DSL has no \
         shape for one: the fetch resolves inside the sacrifice's own ability, \
         so a land destroyed in response still searches"
    ),
    abilities = &[triggered!(
        Trigger::ETB,
        &[
            Effect::SacrificeSelf,
            // NOT SUPPORTED: "When you do" — the fetch is a second, reflexive
            // trigger, which exists only if the sacrifice happened.
            Effect::SearchLibrary {
                filter: &Filter::And(&[
                    Filter::BASIC_LAND,
                    Filter::Or(&[
                        Filter::HasSubtype(land::PLAINS),
                        Filter::HasSubtype(land::ISLAND),
                        Filter::HasSubtype(land::SWAMP),
                    ]),
                ]),
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: false,
            },
            Effect::gain_life(1),
        ]
    )],
);
