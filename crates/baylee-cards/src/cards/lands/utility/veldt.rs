//! Veldt — (no cost) — Land
//! Oracle: This land doesn't untap during your untap step if it has a depletion counter on it.
//! Oracle: At the beginning of your upkeep, remove a depletion counter from this land.
//! Oracle: {T}: Add {G} or {W}. Put a depletion counter on this land.
//! Set: ICE #363 — Ice Age | Scryfall ID: 987534fb-74a9-46a3-805f-fe2fe2df4a90 | Oracle ID: 10b9bfb3-c478-45c6-b227-9c66b63bc79b

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VELDT,
    oracle_id = "10b9bfb3-c478-45c6-b227-9c66b63bc79b",
    scryfall_id = "987534fb-74a9-46a3-805f-fe2fe2df4a90",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(name = "Veldt", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[
            Effect::mana_choice(&[ManaColor::Green, ManaColor::White]),
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
