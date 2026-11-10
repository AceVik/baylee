//! Lava Tubes — (no cost) — Land
//! Oracle: This land doesn't untap during your untap step if it has a depletion counter on it.
//! Oracle: At the beginning of your upkeep, remove a depletion counter from this land.
//! Oracle: {T}: Add {B} or {R}. Put a depletion counter on this land.
//! Set: ICE #358 — Ice Age | Scryfall ID: 5e7c2cf6-f36f-451b-bba5-19a82c659c4c | Oracle ID: 2ffb2647-60bb-4916-b919-cbcf18e5e424

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAVA_TUBES,
    oracle_id = "2ffb2647-60bb-4916-b919-cbcf18e5e424",
    scryfall_id = "5e7c2cf6-f36f-451b-bba5-19a82c659c4c",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(name = "Lava Tubes", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[
            Effect::mana_choice(&[ManaColor::Black, ManaColor::Red]),
            Effect::AddCounter {
                kind: counters::DEPLETION,
                amount: Amount::Fixed(1),
            },
        ]),
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
