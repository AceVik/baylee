//! River Delta — (no cost) — Land
//! Oracle: This land doesn't untap during your untap step if it has a depletion counter on it.
//! Oracle: At the beginning of your upkeep, remove a depletion counter from this land.
//! Oracle: {T}: Add {U} or {B}. Put a depletion counter on this land.
//! Set: ICE #359 — Ice Age | Scryfall ID: ea335fc0-0591-4acd-9ae8-7858222770da | Oracle ID: 4c6c064c-da61-4c7a-9607-8fb4490ff9a6
// PARTIAL — {T} for {U} or {B} and the depletion counter that ability leaves
// behind are both expressible; the two clauses that read that counter back
// are not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RIVER_DELTA,
    oracle_id = "4c6c064c-da61-4c7a-9607-8fb4490ff9a6",
    scryfall_id = "ea335fc0-0591-4acd-9ae8-7858222770da",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(name = "River Delta", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "no conditional static ability (Modifier::DoesNotUntap is unconditional) \
         and no effect that removes a counter (CostPart::RemoveCounterSelf is a cost)",
    ),
    abilities = &[
        // NOT SUPPORTED: "This land doesn't untap during your untap step if it
        // has a depletion counter on it." — a StaticAbility carries no
        // condition, and no variant derives one from a counter count the way
        // AddTypeIfCountersAtLeast does for types.
        // NOT SUPPORTED: "At the beginning of your upkeep, remove a depletion
        // counter from this land." — no Effect removes a counter;
        // RemoveCounterSelf is paid while an ability is activated.
        mana_ability!(
            Cost::TAP,
            &[
                Effect::mana_choice(&[ManaColor::Blue, ManaColor::Black]),
                Effect::AddCounter {
                    kind: counters::DEPLETION,
                    amount: Amount::Fixed(1),
                },
            ]
        ),
    ],
);
