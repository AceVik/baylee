//! Hobbit Hole — (no cost) — Land
//! Oracle: {T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
//! Oracle: Halflingcycling {4} ({4}, Discard this card: Search your library for a Halfling card, reveal it, put it into your hand, then shuffle.)
//! Set: HOB #184 — The Hobbit | Scryfall ID: 0365c439-30bf-4d32-a791-166751bdb996 | Oracle ID: 17492186-9814-4c41-8111-1f000a96c212
// IMPLEMENTED — the Evolving Wilds ability, and Halflingcycling as an
// activated ability from the hand ({4}, discard this card) searching for a
// Halfling card; the reveal and the shuffle are derived by the engine.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::HOBBIT_HOLE,
    oracle_id = "17492186-9814-4c41-8111-1f000a96c212",
    scryfall_id = "0365c439-30bf-4d32-a791-166751bdb996",
    faces = &[face!(name = "Hobbit Hole", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        activated!(
            cost!(TapSelf, SacrificeSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::BATTLEFIELD_TAPPED],
                optional: false,
            }]
        ),
        activated!(
            cost!("{4}", DiscardSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::HasSubtype(creature::HALFLING),
                finds: &[Find::HAND],
                optional: false,
            }],
            zone = ActivationZone::Hand,
        ),
    ],
);
