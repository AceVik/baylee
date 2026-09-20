//! War Room — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}, Pay life equal to the number of colors in your commanders' color identity: Draw a card.
//! Set: SOC #422 — Secrets of Strixhaven Commander | Scryfall ID: 0775b4be-881c-4832-8954-d961064315b6 | Oracle ID: 71c52bf5-2a5d-488e-8b15-7ef290e4b77d
// PARTIAL — the {T}: Add {C} mana ability is built; the draw ability is
// dropped because its cost cannot be said.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WAR_ROOM,
    oracle_id = "71c52bf5-2a5d-488e-8b15-7ef290e4b77d",
    scryfall_id = "0775b4be-881c-4832-8954-d961064315b6",
    faces = &[face!(name = "War Room", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the {3}, {T} draw ability's third cost (pay life equal to the number of colors in your commanders' color identity) has no cost vocabulary"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "{3}, {T}, Pay life equal to the number of colors in your
// commanders' color identity: Draw a card." — CostPart::PayLife takes a
// fixed u16 and CostPart::PayLifeX is the spell's own announced X, so no
// cost part counts a commander's color identity; nor is there an
// Amount variant for that count, so it cannot be paid from the effect side
// either. The ability comes off the card rather than shipping a life payment
// the engine would skip or a number it would guess.
