//! Eater of the Dead — {4}{B} — Creature — Horror
//! Oracle: {0}: If this creature is tapped, exile target creature card from a graveyard and untap this creature.
//! Set: ME1 #67 — Masters Edition | Scryfall ID: 23327a70-5e95-4aa4-824c-cdf3472f5422 | Oracle ID: 3c414ace-fd04-4fac-9379-24f5910f20e4

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EATER_OF_THE_DEAD,
    oracle_id = "3c414ace-fd04-4fac-9379-24f5910f20e4",
    scryfall_id = "23327a70-5e95-4aa4-824c-cdf3472f5422",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Eater of the Dead",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(3),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        Cost::FREE,
        &[Effect::IfCondition {
            condition: Condition::SourceMatches(&Filter::Tapped),
            then: &[
                Effect::Exile {
                    target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::EachPlayer,),
                },
                Effect::UntapSelf,
            ],
            otherwise: &[],
        }],
        target = Some(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::EachPlayer,
        )),
    )],
);
