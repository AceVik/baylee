//! Celestial Sword — {6} — Artifact
//! Oracle: {3}, {T}: Target creature you control gets +3/+3 until end of turn. Its controller sacrifices it at the beginning of the next end step.
//! Set: ME4 #188 — Masters Edition IV | Scryfall ID: bf8b3db8-5e90-4086-a934-cd850b49e249 | Oracle ID: 8f64c3d4-8ed6-43be-9583-33de4b7761fa
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const CREATURE_YOU_CONTROL: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);

card!(
    index = index::CELESTIAL_SWORD,
    oracle_id = "8f64c3d4-8ed6-43be-9583-33de4b7761fa",
    scryfall_id = "bf8b3db8-5e90-4086-a934-cd850b49e249",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Celestial Sword",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(3),
                toughness: Amount::Fixed(3),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::SacrificeEvent],
            },
        ],
        target = Some(TargetSpec::Object(&CREATURE_YOU_CONTROL)),
    )],
);
