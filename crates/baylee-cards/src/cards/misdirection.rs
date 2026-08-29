//! Misdirection — {3}{U}{U} — Instant
//! Oracle: You may exile a blue card from your hand rather than pay this spell's mana cost.
//! Oracle: Change the target of target spell with a single target.
//! Set: DDT #15 — Duel Decks: Merfolk vs. Goblins | Scryfall ID: c96763d6-0cea-40ed-afb2-886bfebe50a0 | Oracle ID: c39e5fb0-6de3-4105-ad3c-0ecb8951a1d5
// IMPLEMENTED — pitch cast + target redirection.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, AltCondition, AlternativeCost, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_SPELL: Filter = Filter::Any;
static BLUE_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Blue]));

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(96),
    oracle_id: "c39e5fb0-6de3-4105-ad3c-0ecb8951a1d5",
    scryfall_id: "c96763d6-0cea-40ed-afb2-886bfebe50a0",
    faces: &[FaceDef {
        name: "Misdirection",
        mana_cost: baylee_core::mana!("{3}{U}{U}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[AlternativeCost {
            cost: Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::ExileFromHand(&BLUE_CARD)],
            },
            condition: AltCondition::Always,
        }],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::RedirectTarget {
            new_filter: &Filter::Any,
        }],
        targets: Some(TargetReq::one(TargetSpec::Spell(&ANY_SPELL))),
    }],
};

#[cfg(test)]
mod tests {}
