//! Duplicant — {6} — Artifact Creature — Shapeshifter
//! Oracle: Imprint — When this creature enters, you may exile target nontoken creature.
//! Oracle: As long as a card exiled with this creature is a creature card, this creature has the power, toughness, and creature types of the last creature card exiled with it. It's still a Shapeshifter.
//! Set: CMM #946 — Commander Masters | Scryfall ID: 9ecb184f-c282-42f3-9576-09781a116352 | Oracle ID: ea86abfa-6cab-4ef0-8463-34136fc25b59
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DUPLICANT,
    oracle_id = "ea86abfa-6cab-4ef0-8463-34136fc25b59",
    scryfall_id = "9ecb184f-c282-42f3-9576-09781a116352",
    faces = &[face!(
        name = "Duplicant",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(2),
        toughness = Some(4),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::MayDo {
                effects: &[Effect::ExileTargetsWithSource],
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(
                &Filter::NONTOKEN_CREATURE
            ))),
        ),
        static_ability!(Filter::This, Modifier::CopyLastExiledCreaturePTAndSubtypes),
    ],
);
