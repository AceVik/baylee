//! Urza's Power Plant — (no cost) — Land — Urza's Power-Plant
//! Oracle: {T}: Add {C}. If you control an Urza's Mine and an Urza's Tower, add {C}{C} instead.
//! Set: CMM #1052 — Commander Masters | Scryfall ID: b0449a19-37f7-4169-9e32-928db5ec76fe | Oracle ID: e11966cd-2ee3-4df4-b099-abf42dcdf0db
// PARTIAL — the plain {T}: Add {C} is built; the "instead" clause is not sayable.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URZA_S_POWER_PLANT,
    oracle_id = "e11966cd-2ee3-4df4-b099-abf42dcdf0db",
    scryfall_id = "b0449a19-37f7-4169-9e32-928db5ec76fe",
    faces = &[face!(
        name = "Urza's Power Plant",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::URZA_S, subtypes::land::POWER_PLANT],
    ),],
    coverage = Coverage::Partial(
        "the \"add {C}{C} instead\" mana line: no construct asks for two different \
         permanents at once — Condition::ControlCount takes a single filter, so \
         Or(Urza's Mine, Urza's Tower) counted at 2 also accepts two Mines, and \
         Effect::AddMana has no conditional amount"
    ),
    // NOT SUPPORTED: "If you control an Urza's Mine and an Urza's Tower, add {C}{C} instead."
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
