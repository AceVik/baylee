//! Ivory Tower — {1} — Artifact
//! Oracle: At the beginning of your upkeep, you gain X life, where X is the number of cards in your hand minus 4.
//! Set: VMA #269 — Vintage Masters | Scryfall ID: aaf91af0-0243-4dca-90de-d7580f4f9d38 | Oracle ID: d416a4ed-9f16-4a8b-8aee-03c630e1eb6c
// IMPLEMENTED — your upkeep gains 4 fewer than the cards in your hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::IVORY_TOWER,
    oracle_id = "d416a4ed-9f16-4a8b-8aee-03c630e1eb6c",
    scryfall_id = "aaf91af0-0243-4dca-90de-d7580f4f9d38",
    faces = &[face!(
        name = "Ivory Tower",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::GainLife {
            amount: Amount::SaturatingSub {
                base: &Amount::CountOf {
                    filter: &Filter::Any,
                    zone: ZoneSel::HandYou,
                },
                subtract: 4,
            },
        }]
    )],
);
