//! Urza's Mine — (no cost) — Land — Urza's Mine
//! Oracle: {T}: Add {C}. If you control an Urza's Power-Plant and an Urza's Tower, add {C}{C} instead.
//! Set: CMM #1051 — Commander Masters | Scryfall ID: 396bbb7d-ae61-4d8d-b931-9ed2f712832e | Oracle ID: 33e85a8a-86df-4cdc-a9cc-8cbabe92c3c0
// PARTIAL — {T}: Add {C} is built exactly; the "{C}{C} instead" clause is
// left off the card, because its condition cannot be stated (see below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URZA_S_MINE,
    oracle_id = "33e85a8a-86df-4cdc-a9cc-8cbabe92c3c0",
    scryfall_id = "396bbb7d-ae61-4d8d-b931-9ed2f712832e",
    faces = &[face!(
        name = "Urza's Mine",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::URZA_S, subtypes::land::MINE],
    ),],
    coverage = Coverage::Partial(
        "the {C}{C} clause's condition is a conjunction — an Urza's Power-Plant AND an Urza's Tower — and Condition has no conjunction: ControlCount counts one filter"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "If you control an Urza's Power-Plant and an Urza's Tower,
// add {C}{C} instead."
//
// The one construct that exists for a printed "if you control …" on a mana
// ability is `Condition::ControlCount(&filter, n)`, and it asks its question
// of a single filter: "you control at least N permanents matching this". That
// can say "two of these" but not "one of each of these two kinds", so the
// nearest writable filter — `Or(&[POWER_PLANT, TOWER])` at count two — is
// already true under two Urza's Power-Plants, and the land would make {C}{C}
// on a board the card says nothing about. Writing that would be shipping a
// wrong clause, so the clause comes off the card instead: this land taps for
// {C} and never for {C}{C}.
