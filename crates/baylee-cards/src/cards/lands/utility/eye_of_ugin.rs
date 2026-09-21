//! Eye of Ugin — (no cost) — Legendary Land
//! Oracle: Colorless Eldrazi spells you cast cost {2} less to cast.
//! Oracle: {7}, {T}: Search your library for a colorless creature card, reveal it, put it into your hand, then shuffle.
//! Set: MM2 #242 — Modern Masters 2015 | Scryfall ID: d2d5124b-4d73-4aa9-9331-88e03779ffad | Oracle ID: 10d13ff6-c4d0-4753-8939-a8a90f0e92bb
// PARTIAL — the {7}, {T} tutor is built (the search's own rules supply the
// reveal and the shuffle); the static cost reducer is not sayable.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EYE_OF_UGIN,
    oracle_id = "10d13ff6-c4d0-4753-8939-a8a90f0e92bb",
    scryfall_id = "d2d5124b-4d73-4aa9-9331-88e03779ffad",
    faces = &[face!(
        name = "Eye of Ugin",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial("colorless Eldrazi spells you cast cost {2} less to cast"),
    abilities = &[activated!(
        cost!("{7}", TapSelf),
        &[Effect::SearchLibrary {
            filter: &f!(colorless CREATURE),
            finds: &[Find::HAND],
            optional: false,
        }]
    ),],
);

// NOT SUPPORTED: "Colorless Eldrazi spells you cast cost {2} less to cast" —
// CostReduction's only variant is NotStartingPlayer(n), and no Modifier
// states a reduction on a cost.
