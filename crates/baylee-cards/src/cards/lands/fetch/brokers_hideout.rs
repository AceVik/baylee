//! Brokers Hideout — (no cost) — Land
//! Oracle: When this land enters, sacrifice it. When you do, search your library for a basic Forest, Plains, or Island card, put it onto the battlefield tapped, then shuffle and you gain 1 life.
//! Set: SNC #248 — Streets of New Capenna | Scryfall ID: 989b299b-daa9-4bda-94e2-9a2f0e8f2bce | Oracle ID: bd002797-a545-4bee-88bf-b878436e7cca
// IMPLEMENTED — the enter trigger sacrifices this land, and the printed "when
// you do" is this land's own leaves-the-battlefield trigger: search up a basic
// Forest, Plains or Island onto the battlefield tapped (the shuffle after a
// search is derived) and gain 1 life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

// "a basic Forest, Plains, or Island card" — the Basic supertype (CR 205.4a)
// and one of the three printed land types, in the order the sentence names
// them.
static BASIC_FOREST_PLAINS_OR_ISLAND: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
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
    abilities = &[
        triggered!(Trigger::ETB, &[Effect::SacrificeSelf]),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[
                Effect::SearchLibrary {
                    filter: &BASIC_FOREST_PLAINS_OR_ISLAND,
                    finds: &[Find::BATTLEFIELD_TAPPED],
                    optional: false,
                },
                Effect::gain_life(1),
            ]
        ),
    ],
);
