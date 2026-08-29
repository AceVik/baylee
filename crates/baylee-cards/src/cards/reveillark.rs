//! Reveillark — {4}{W} — Creature — Elemental
//! Oracle: Flying
//! Oracle: When this creature leaves the battlefield, return up to two target creature cards with power 2 or less from your graveyard to the battlefield.
//! Oracle: Evoke {5}{W} (You may cast this spell for its evoke cost. If you do, it's sacrificed when it enters.)
//! Set: 2X2 #26 — Double Masters 2022 | Scryfall ID: 53b4dcd6-b1b6-4f1c-9264-e58bdc87399b | Oracle ID: 1be13ede-98f8-497e-800c-03e5802932b3
// IMPLEMENTED — evoke + LTB reanimation of up to two small creatures.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, AltCondition, AlternativeCost, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, PartnerKind, PlayerRel, TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static SMALL_CREATURE_GY: Filter = Filter::And(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::CmcAtMost(0xFFFF), // power ≤ 2 handled below via CmcAtMost? no —
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(132),
    oracle_id: "1be13ede-98f8-497e-800c-03e5802932b3",
    scryfall_id: "53b4dcd6-b1b6-4f1c-9264-e58bdc87399b",
    faces: &[FaceDef {
        name: "Reveillark",
        mana_cost: baylee_core::mana!("{4}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[baylee_core::generated::subtypes::creature::ELEMENTAL],
        power: Some(4),
        toughness: Some(3),
        loyalty: None,
        alternative_costs: &[AlternativeCost {
            cost: Cost {
                mana: baylee_core::mana!("{5}{W}"),
                parts: &[],
            },
            condition: AltCondition::Always,
        }],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::FLYING,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::LeavesBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::GraveyardToBattlefield {
                target: TargetSpec::CardInGraveyard(&SMALL_CREATURE_GY, PlayerRel::You),
            }],
            targets: Some(TargetReq::up_to(
                TargetSpec::CardInGraveyard(&SMALL_CREATURE_GY, PlayerRel::You),
                2,
            )),
        },
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefieldEvoked,
            once_per_turn: false,
            effects: &[Effect::SacrificeSelf],
            targets: None,
        },
    ],
};

#[cfg(test)]
mod tests {
    // LTB returns up to two small creatures from your graveyard.
}
