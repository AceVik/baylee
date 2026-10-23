//! Thran Turbine — {1} — Artifact
//! Oracle: At the beginning of your upkeep, you may add {C}{C}. This mana can't be spent to cast spells.
//! Set: USG #311 — Urza's Saga | Scryfall ID: 791c310f-b50d-4993-9add-9f585f6172af | Oracle ID: 6c31f06d-bf02-48a7-9c17-be2a226b8eae
// PARTIAL: mana spend restriction 'can't be spent to cast spells' is not in ManaSpendRestriction; at the beginning of your upkeep, you may add {C}{C}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THRAN_TURBINE,
    oracle_id = "6c31f06d-bf02-48a7-9c17-be2a226b8eae",
    scryfall_id = "791c310f-b50d-4993-9add-9f585f6172af",
    coverage = Coverage::Partial(
        "mana spend restriction 'can't be spent to cast spells' is not in ManaSpendRestriction; at the beginning of your upkeep, you may add {C}{C}",
    ),
    faces = &[face!(
        name = "Thran Turbine",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::mana(ManaColor::Colorless, 2)],
        }],
    ),],
);
