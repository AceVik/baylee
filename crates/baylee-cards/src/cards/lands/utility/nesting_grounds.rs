//! Nesting Grounds — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Move a counter from target permanent you control onto a second target permanent. Activate only as a sorcery.
//! Set: ECC #155 — Lorwyn Eclipsed Commander | Scryfall ID: c25ad164-4076-496d-833c-d607391b1ac8 | Oracle ID: d27bb97d-286b-4947-8d7b-443e4df93319
// PARTIAL — {T}: Add {C} is implemented; the counter-moving ability is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NESTING_GROUNDS,
    oracle_id = "d27bb97d-286b-4947-8d7b-443e4df93319",
    scryfall_id = "c25ad164-4076-496d-833c-d607391b1ac8",
    faces = &[face!(name = "Nesting Grounds", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no Effect variant moves a counter from one permanent to another, so the {1}, {T} ability is not implemented",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{1}, {T}: Move a counter from target permanent you
        // control onto a second target permanent. Activate only as a
        // sorcery." — nothing in Effect takes a counter *off* a permanent
        // (`AddCounter`/`AddCounterFilter` only put them on, and
        // `RemoveCounterSelf` is a cost paid by the source), nothing lets the
        // resolving effect choose which counter kind to move, and the clause
        // names two targets with different filters rather than one.
    ],
);
