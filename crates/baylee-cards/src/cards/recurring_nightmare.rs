//! Recurring Nightmare — {2}{B} — Enchantment
//! Oracle: Sacrifice a creature, Return this enchantment to its owner's hand: Return target creature card from your graveyard to the battlefield. Activate only as a sorcery.
//! Set: TPR #113 — Tempest Remastered | Scryfall ID: b50e1800-a45c-43bd-8886-8a06145d9346 | Oracle ID: a6708b11-1bcd-4208-a967-fe91f2e3313c
// IMPLEMENTED — sacrifice + bounce-to-hand cost, sorcery-speed
// reanimation.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, PlayerRel, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE_YOU_CONTROL: Filter =
    Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::ControlledByYou]);
static CREATURE_CARD: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(127),
    oracle_id: "a6708b11-1bcd-4208-a967-fe91f2e3313c",
    scryfall_id: "b50e1800-a45c-43bd-8886-8a06145d9346",
    faces: &[FaceDef {
        name: "Recurring Nightmare",
        mana_cost: baylee_core::mana!("{2}{B}"),
        types: TypeSet::ENCHANTMENT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
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
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::ZERO,
            parts: &[
                CostPart::Sacrifice(&CREATURE_YOU_CONTROL),
                CostPart::ReturnSelfToHand,
            ],
        },
        effects: &[Effect::GraveyardToBattlefield {
            target: TargetSpec::CardInGraveyard(&CREATURE_CARD, PlayerRel::You),
        }],
        target: Some(TargetSpec::CardInGraveyard(&CREATURE_CARD, PlayerRel::You)),
        timing: ActivationTiming::SorcerySpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
    }],
};

#[cfg(test)]
mod tests {}
