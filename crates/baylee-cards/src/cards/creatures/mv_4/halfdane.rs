//! Halfdane — {1}{W}{U}{B} — Legendary Creature — Shapeshifter
//! Oracle: At the beginning of your upkeep, Halfdane's base power and toughness become equal to the power and toughness of target creature other than Halfdane until the end of your next upkeep.
//! Set: ME3 #150 — Masters Edition III | Scryfall ID: cac1e57a-0ef5-442b-a492-059fc6ff3dfc | Oracle ID: 2ee13155-a3b8-4cad-8e68-5d2c5aba3bb4

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HALFDANE,
    oracle_id = "2ee13155-a3b8-4cad-8e68-5d2c5aba3bb4",
    scryfall_id = "cac1e57a-0ef5-442b-a492-059fc6ff3dfc",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Halfdane",
        mana_cost = mana!("{1}{W}{U}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::SetSourcePTFromTarget {
            duration: Duration::UntilEndOfYourNextUpkeep
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::And(&[
            Filter::CREATURE,
            Filter::Another
        ])))),
    )],
);
