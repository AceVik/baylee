//! Shapeshifter — {6} — Artifact Creature — Shapeshifter
//! Oracle: As this creature enters, choose a number between 0 and 7.
//! Oracle: At the beginning of your upkeep, you may choose a number between 0 and 7.
//! Oracle: Shapeshifter's power is equal to the last chosen number and its toughness is equal to 7 minus that number.
//! Set: ME4 #226 — Masters Edition IV | Scryfall ID: f15f2638-3895-459a-84af-fb91de06c395 | Oracle ID: 82a6d89d-9215-4540-b7d5-26cdd6afb05b
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHAPESHIFTER,
    oracle_id = "82a6d89d-9215-4540-b7d5-26cdd6afb05b",
    scryfall_id = "f15f2638-3895-459a-84af-fb91de06c395",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shapeshifter",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(0),
        toughness = Some(7),
        enter_modifiers = &[EnterModifier::ChooseNumber { min: 0, max: 7 }],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You
            },
            &[Effect::MayDo {
                effects: &[Effect::ChooseNumber { min: 0, max: 7 }]
            }]
        ),
        static_ability!(
            Filter::This,
            Modifier::CharacteristicPTSeparate {
                power: PtCount::ChosenNumber,
                toughness: PtCount::Complement {
                    from: 7,
                    count: &PtCount::ChosenNumber
                },
            }
        ),
    ],
);
