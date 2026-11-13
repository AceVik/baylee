//! Initiates of the Ebon Hand — {B} — Creature — Cleric
//! Oracle: {1}: Add {B}. If this ability has been activated four or more times this turn, sacrifice this creature at the beginning of the next end step.
//! Set: 5ED #169 — Fifth Edition | Scryfall ID: c83085df-1ffb-4178-9fc0-fd347196673f | Oracle ID: 0bf359ef-669f-41dc-8ab5-ba8e58b2f398
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INITIATES_OF_THE_EBON_HAND,
    oracle_id = "0bf359ef-669f-41dc-8ab5-ba8e58b2f398",
    scryfall_id = "c83085df-1ffb-4178-9fc0-fd347196673f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Initiates of the Ebon Hand",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[mana_ability!(
        cost!("{1}"),
        &[
            Effect::mana(ManaColor::Black, 1),
            Effect::IfActivatedThisTurnAtLeast {
                n: 4,
                then: &[Effect::AtNextEndStep {
                    effects: &[Effect::SacrificeObject {
                        target: TargetSpec::EventObject,
                    }],
                }],
            },
        ],
    ),],
);
