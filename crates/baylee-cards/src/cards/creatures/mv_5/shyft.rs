//! Shyft — {4}{U} — Creature — Shapeshifter
//! Oracle: At the beginning of your upkeep, you may have this creature become the color or colors of your choice. (This effect lasts indefinitely.)
//! Set: ME2 #66 — Masters Edition II | Scryfall ID: d7f2bbf6-65b7-41c4-a219-f8ea4aeb9bbc | Oracle ID: 40c3007a-be6c-4400-9139-12b9ca098ac4
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHYFT,
    oracle_id = "40c3007a-be6c-4400-9139-12b9ca098ac4",
    scryfall_id = "d7f2bbf6-65b7-41c4-a219-f8ea4aeb9bbc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Shyft",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(4),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::MayDo {
            effects: &[Effect::ChooseColors {
                subject: ColorSubject::Source,
                max: 5,
                duration: Duration::Indefinitely,
            }],
        }],
    )],
);
