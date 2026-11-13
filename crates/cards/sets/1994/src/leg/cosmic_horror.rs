//! Cosmic Horror — {3}{B}{B}{B} — Creature — Horror
//! Oracle: First strike
//! Oracle: At the beginning of your upkeep, destroy this creature unless you pay {3}{B}{B}{B}. If this creature is destroyed this way, it deals 7 damage to you.
//! Set: ME3 #61 — Masters Edition III | Scryfall ID: 64288873-d3bf-49f6-b0e4-a1e712f8ba58 | Oracle ID: d73df13d-942a-4104-ac9d-de7c100c086f
// IMPLEMENTED — first strike; at your upkeep, pay {3}{B}{B}{B} or the Horror is
// destroyed and then deals 7 damage to you if it is in a graveyard.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COSMIC_HORROR,
    oracle_id = "d73df13d-942a-4104-ac9d-de7c100c086f",
    scryfall_id = "64288873-d3bf-49f6-b0e4-a1e712f8ba58",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cosmic Horror",
        mana_cost = mana!("{3}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORROR],
        power = Some(7),
        toughness = Some(7),
    ),],
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You
        },
        &[Effect::PlayerMayPayManaOr {
            player: PlayerRel::You,
            cost: mana!("{3}{B}{B}{B}"),
            effect: &Effect::Sequence(&[
                Effect::destroy(TargetSpec::ThisObject),
                Effect::IfCondition {
                    condition: Condition::SourceMatches(&Filter::InZone(ZoneRef::Graveyard)),
                    then: &[Effect::DealDamage {
                        amount: Amount::Fixed(7),
                        target: TargetSpec::Player(PlayerRel::You)
                    }],
                    otherwise: &[]
                }
            ])
        }]
    )],
);
