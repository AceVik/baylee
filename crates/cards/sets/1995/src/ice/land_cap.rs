//! Land Cap — (no cost) — Land
//! Oracle: This land doesn't untap during your untap step if it has a depletion counter on it.
//! Oracle: At the beginning of your upkeep, remove a depletion counter from this land.
//! Oracle: {T}: Add {W} or {U}. Put a depletion counter on this land.
//! Set: ICE #357 — Ice Age | Scryfall ID: c4806c02-7a4d-42e3-affd-0338084bd3ab | Oracle ID: bfec4d0a-3792-4bc3-bae1-e639da5bb9a6

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAND_CAP,
    oracle_id = "bfec4d0a-3792-4bc3-bae1-e639da5bb9a6",
    scryfall_id = "c4806c02-7a4d-42e3-affd-0338084bd3ab",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[face!(name = "Land Cap", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[
            Effect::mana_choice(&[ManaColor::White, ManaColor::Blue]),
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
