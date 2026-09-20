//! Roadside Reliquary — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice this land: Draw a card if you control an artifact. Draw a card if you control an enchantment.
//! Set: NEO #272 — Kamigawa: Neon Dynasty | Scryfall ID: 8002de90-93fb-48ea-a849-40fdad0aef5a | Oracle ID: 2fb13687-0518-4ba0-a5ae-dd609464b026
// PARTIAL — {T}: Add {C} is built; the sacrifice ability is not expressible
// (the two conditional draws — see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ROADSIDE_RELIQUARY,
    oracle_id = "2fb13687-0518-4ba0-a5ae-dd609464b026",
    scryfall_id = "8002de90-93fb-48ea-a849-40fdad0aef5a",
    faces = &[face!(name = "Roadside Reliquary", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {2}, {T}, Sacrifice ability is missing: no Effect branches on a board state, so neither \"draw a card if you control an artifact\" nor the enchantment half is sayable"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: {2}, {T}, Sacrifice this land: Draw a card if you
        // control an artifact. Draw a card if you control an enchantment. —
        // the cost is ordinary, but a Condition gates an activation or a
        // printed intervening `if`, and no Effect readable inside an effect
        // list asks about the board.
    ],
);
