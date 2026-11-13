//! Bone Shredder — {2}{B} — Creature — Phyrexian Minion
//! Oracle: Flying
//! Oracle: Echo {2}{B} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters, destroy target nonartifact, nonblack creature.
//! Set: MOC #247 — March of the Machine Commander | Scryfall ID: 7165a9ac-3f44-4d14-93bb-d4d4970ab41b | Oracle ID: 69aa1ac7-17da-4fc9-a3dc-df8a52841f4a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const NONARTIFACT_NONBLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::LacksType(TypeSet::ARTIFACT),
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);

card!(
    index = index::BONE_SHREDDER,
    oracle_id = "69aa1ac7-17da-4fc9-a3dc-df8a52841f4a",
    scryfall_id = "7165a9ac-3f44-4d14-93bb-d4d4970ab41b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bone Shredder",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::MINION],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{2}{B}"),
        },
        triggered!(
            Trigger::ETB,
            &[Effect::destroy(TargetSpec::Object(
                &NONARTIFACT_NONBLACK_CREATURE,
            ))],
            targets = Some(TargetReq::one(TargetSpec::Object(
                &NONARTIFACT_NONBLACK_CREATURE,
            ))),
        ),
    ],
);
