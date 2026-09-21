//! HELIOS One — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: You get {E} (an energy counter).
//! Oracle: {3}, {T}, Pay X {E}, Sacrifice this land: Destroy target nonland permanent with mana value X. Activate only as a sorcery.
//! Set: PIP #149 — Fallout | Scryfall ID: 4102d28e-437b-440b-bf9b-8b4f6fb85a6c | Oracle ID: cfb1a656-0bf1-484d-b099-33087914250b
// PARTIAL — tap for {C} is built; the two energy abilities are dropped because
// the DSL cannot put counters on players nor pay energy as a cost.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HELIOS_ONE,
    oracle_id = "cfb1a656-0bf1-484d-b099-33087914250b",
    scryfall_id = "4102d28e-437b-440b-bf9b-8b4f6fb85a6c",
    coverage = Coverage::Partial(
        "no effect puts a counter on a player and CostPart has no variant for paying energy counters",
    ),
    faces = &[face!(name = "HELIOS One", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: {1}, {T}: You get {E} (an energy counter).
        // NOT SUPPORTED: {3}, {T}, Pay X {E}, Sacrifice this land: Destroy target nonland permanent with mana value X. Activate only as a sorcery.
    ],
);
