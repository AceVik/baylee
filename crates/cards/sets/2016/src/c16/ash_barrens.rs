//! Ash Barrens — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: Basic landcycling {1} ({1}, Discard this card: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.)
//! Set: TMC #60 — Teenage Mutant Ninja Turtles Eternal | Scryfall ID: e8510205-4baa-4f1c-bf64-5d931e2ddf48 | Oracle ID: 58257464-278e-45fa-8e0b-bcd9a7500bc1
// IMPLEMENTED — {T}: Add {C}, plus basic landcycling {1} as a hand-zone
// activation: discard this card, search a basic land into hand (the reveal
// and the shuffle are derived from the search, not declared).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASH_BARRENS,
    oracle_id = "58257464-278e-45fa-8e0b-bcd9a7500bc1",
    scryfall_id = "e8510205-4baa-4f1c-bf64-5d931e2ddf48",
    faces = &[face!(name = "Ash Barrens", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", DiscardSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::HAND],
                optional: false,
            }],
            zone = ActivationZone::Hand
        ),
    ],
);
