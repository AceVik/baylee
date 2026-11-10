//! Timberline Ridge — (no cost) — Land
//! Oracle: This land doesn't untap during your untap step if it has a depletion counter on it.
//! Oracle: At the beginning of your upkeep, remove a depletion counter from this land.
//! Oracle: {T}: Add {R} or {G}. Put a depletion counter on this land.
//! Set: ICE #361 — Ice Age | Scryfall ID: 87cc2fc9-0a24-4ac1-afcc-9317b90c7178 | Oracle ID: a597d2b7-484b-4cfd-89a6-d166cb1a3420

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TIMBERLINE_RIDGE,
    oracle_id = "a597d2b7-484b-4cfd-89a6-d166cb1a3420",
    scryfall_id = "87cc2fc9-0a24-4ac1-afcc-9317b90c7178",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(name = "Timberline Ridge", types = TypeSet::LAND,),],
    abilities = &[
        mana_ability!(
            Cost::TAP,
            &[
                Effect::mana_choice(&[ManaColor::Red, ManaColor::Green]),
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
