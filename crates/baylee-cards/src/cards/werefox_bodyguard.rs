//! Werefox Bodyguard — {1}{W}{W} — Creature — Elf Fox Knight
//! Oracle: Flash
//! Oracle: When this creature enters, exile up to one other target non-Fox creature until this creature leaves the battlefield.
//! Oracle: {1}{W}, Sacrifice this creature: You gain 2 life.
//! Set: WOE #39 — Wilds of Eldraine | Scryfall ID: 4494dfa1-1343-417e-b0c5-2b096442dd0e | Oracle ID: d5ee2ced-29f4-430f-962e-2f930b92624c
// IMPLEMENTED — flash, linked-exile ETB, sacrifice-for-life outlet.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, CostPart,
    Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind, TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static OTHER_NON_FOX_CREATURE: Filter = Filter::And(&[
    Filter::Another,
    Filter::HasType(TypeSet::CREATURE),
    Filter::Not(&Filter::HasSubtype(creature::FOX)),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(190),
    oracle_id: "d5ee2ced-29f4-430f-962e-2f930b92624c",
    scryfall_id: "4494dfa1-1343-417e-b0c5-2b096442dd0e",
    faces: &[FaceDef {
        name: "Werefox Bodyguard",
        mana_cost: baylee_core::mana!("{1}{W}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::ELF, creature::FOX, creature::KNIGHT],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::FLASH,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::ExileLinked {
                target: TargetSpec::Object(&OTHER_NON_FOX_CREATURE),
            }],
            targets: Some(TargetReq {
                spec: TargetSpec::Object(&OTHER_NON_FOX_CREATURE),
                min: 0,
                max: 1,
                count_is_x: false,
            }),
        },
        AbilityDef::Activated {
            cost: Cost {
                mana: baylee_core::mana!("{1}{W}"),
                parts: &[CostPart::SacrificeSelf],
            },
            effects: &[Effect::GainLife {
                amount: Amount::Fixed(2),
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
