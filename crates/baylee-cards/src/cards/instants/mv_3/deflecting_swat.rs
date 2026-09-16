//! Deflecting Swat — {2}{R} — Instant
//! Oracle: If you control a commander, you may cast this spell without paying its mana cost.
//! Oracle: You may choose new targets for target spell or ability.
//! Set: CMM #214 — Commander Masters | Scryfall ID: b4b36435-55b3-4615-8812-af41d4fc64d9 | Oracle ID: ae120613-97d6-4393-b39d-c3e6c076f5d6
// IMPLEMENTED — cast for free while you control a commander, then choose new
// targets for a target spell or ability.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEFLECTING_SWAT,
    oracle_id = "ae120613-97d6-4393-b39d-c3e6c076f5d6",
    scryfall_id = "b4b36435-55b3-4615-8812-af41d4fc64d9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Deflecting Swat",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
        alternative_costs = &[AlternativeCost {
            cost: Cost::FREE,
            condition: AltCondition::CommanderControlled,
        }],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::RedirectTarget {
            new_filter: &Filter::Any,
        }],
        targets = Some(TargetReq::one(TargetSpec::SpellOrAbility(&Filter::Any)))
    )],
);
