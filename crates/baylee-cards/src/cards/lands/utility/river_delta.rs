//! River Delta — (no cost) — Land
//! Oracle: This land doesn't untap during your untap step if it has a depletion counter on it.
//! Oracle: At the beginning of your upkeep, remove a depletion counter from this land.
//! Oracle: {T}: Add {U} or {B}. Put a depletion counter on this land.
//! Set: ICE #359 — Ice Age | Scryfall ID: ea335fc0-0591-4acd-9ae8-7858222770da | Oracle ID: 4c6c064c-da61-4c7a-9607-8fb4490ff9a6

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RIVER_DELTA,
    oracle_id = "4c6c064c-da61-4c7a-9607-8fb4490ff9a6",
    scryfall_id = "ea335fc0-0591-4acd-9ae8-7858222770da",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    faces = &[face!(name = "River Delta", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
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
        static_ability!(
            Filter::This,
            Modifier::DoesNotUntap,
            condition = Some(Condition::CountersOnSelf(counters::DEPLETION, 1))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::RemoveCounterSelf {
                kind: counters::DEPLETION,
                n: 1
            }]
        ),
    ],
);
