//! Solitude — {3}{W}{W} — Creature — Elemental Incarnation
//! Oracle: Flash
//! Oracle: Lifelink
//! Oracle: When this creature enters, exile up to one other target creature. That creature's controller gains life equal to its power.
//! Oracle: Evoke—Exile a white card from your hand.
//! Set: MSC #37 — Marvel Super Heroes Commander | Scryfall ID: 47a6234f-309f-4e03-9263-66da48b57153 | Oracle ID: dcb9c2a7-ae54-4ddc-a567-640bf4bf4366
// IMPLEMENTED — flash/lifelink, exile ETB with life, pitch-evoke.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, AltCondition, AlternativeCost, Amount, CardDef, CommanderRule, Cost, CostPart,
    Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind, PlayerRel, TargetReq, TargetSpec,
    Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE_OTHER: Filter = Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::Another]);
static WHITE_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::White]));

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(152),
    oracle_id: "dcb9c2a7-ae54-4ddc-a567-640bf4bf4366",
    scryfall_id: "47a6234f-309f-4e03-9263-66da48b57153",
    faces: &[FaceDef {
        name: "Solitude",
        mana_cost: baylee_core::mana!("{3}{W}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            baylee_core::generated::subtypes::creature::ELEMENTAL,
            baylee_core::generated::subtypes::creature::INCARNATION,
        ],
        power: Some(3),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[AlternativeCost {
            cost: Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::ExileFromHand(&WHITE_CARD)],
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
        disturb: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::FLASH.union(KeywordSet::LIFELINK),
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[
                Effect::Exile {
                    target: TargetSpec::Object(&CREATURE_OTHER),
                },
                Effect::GainLifeFor {
                    amount: Amount::TargetPower,
                    who: PlayerRel::ControllerOfTarget,
                },
            ],
            targets: Some(TargetReq::up_to_one(TargetSpec::Object(&CREATURE_OTHER))),
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
    // Pitch path: exile a white card from hand, no mana spent; creature is
    // sacrificed after its ETB.
}
