//! Pact of Negation — {0} — Instant
//! Oracle: Counter target spell.
//! Oracle: At the beginning of your next upkeep, pay {3}{U}{U}. If you don't, you lose the game.
//! Set: TSR #77 — Time Spiral Remastered | Scryfall ID: 1ed4c0bb-b710-44a1-b8bc-6bd11c27b8b8 | Oracle ID: f3e213a4-ba5a-468a-93b3-c0a34e1bd725
// IMPLEMENTED — zero-cost counter + delayed pay-or-lose upkeep.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_SPELL: Filter = Filter::Any;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(107),
    oracle_id: "f3e213a4-ba5a-468a-93b3-c0a34e1bd725",
    scryfall_id: "1ed4c0bb-b710-44a1-b8bc-6bd11c27b8b8",
    faces: &[FaceDef {
        name: "Pact of Negation",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::INSTANT,
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
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[
            Effect::CounterTargetSpell,
            Effect::PayCostOrLoseLater {
                cost: baylee_core::mana!("{3}{U}{U}"),
            },
        ],
        targets: Some(TargetReq::one(TargetSpec::Spell(&ANY_SPELL))),
    }],
};

#[cfg(test)]
mod tests {}
