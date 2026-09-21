//! Hidden Hideout — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add one mana of any color in your commander's color identity.
//! Oracle: {2}, {T}: Target creature you control with a counter on it gains lifelink until end of turn.
//! Set: TMC #43 — Teenage Mutant Ninja Turtles Eternal | Scryfall ID: f356522c-018d-43c7-988a-9ef9829ae75c | Oracle ID: b639e1fe-d099-4cab-a0d0-a1b33c7f31dd
// IMPLEMENTED — enters tapped; {T} adds one mana of any color in your
// commander's color identity. The counter-restricted lifelink activation is
// left off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HIDDEN_HIDEOUT,
    oracle_id = "b639e1fe-d099-4cab-a0d0-a1b33c7f31dd",
    scryfall_id = "f356522c-018d-43c7-988a-9ef9829ae75c",
    faces = &[face!(
        name = "Hidden Hideout",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "target creature you control with a counter on it: no Filter variant matches an object \
         that has a counter on it, so the printed targeting restriction cannot be stated"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana_commander_identity()]),
        // NOT SUPPORTED: "{2}, {T}: Target creature you control with a counter
        // on it gains lifelink until end of turn." The pump itself is
        // expressible (PumpTarget with KeywordSet::LIFELINK), but a target of
        // "a creature you control **with a counter on it**" is not: Filter has
        // no predicate over an object's counters. The ability is left off
        // rather than widened to every creature you control, which would let
        // the card target a permanent the printing forbids.
    ],
);
