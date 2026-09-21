//! Avishkar Raceway — (no cost) — Land
//! Oracle: Start your engines! (If you have no speed, it starts at 1. It increases once on each of your turns when an opponent loses life. Max speed is 4.)
//! Oracle: {T}: Add {C}.
//! Oracle: Max speed — {3}, {T}, Discard a card: Draw a card.
//! Set: DFT #249 — Aetherdrift | Scryfall ID: 08a6b378-c7fa-4226-a310-4ee7e550b4d6 | Oracle ID: e2c35551-1ba5-4424-baf9-821b49bbcc8c
// PARTIAL — {T}: Add {C}; the speed mechanic and its Max speed gate are not expressible.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "Start your engines!" — speed is a player attribute with no CounterKind and no Condition in the DSL.
// NOT SUPPORTED: "Max speed — {3}, {T}, Discard a card: Draw a card." — no Condition can gate on speed, so written unconditionally the ability would be offered at every speed.

card!(
    index = index::AVISHKAR_RACEWAY,
    oracle_id = "e2c35551-1ba5-4424-baf9-821b49bbcc8c",
    scryfall_id = "08a6b378-c7fa-4226-a310-4ee7e550b4d6",
    faces = &[face!(name = "Avishkar Raceway", types = TypeSet::LAND,),],
    coverage = Coverage::Partial("speed (Start your engines! / Max speed) has no DSL vocabulary"),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
