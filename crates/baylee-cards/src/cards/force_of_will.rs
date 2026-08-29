//! Force of Will — {3}{U}{U} — Instant
//! Oracle: You may pay 1 life and exile a blue card from your hand rather than pay this spell's mana cost.
//! Oracle: Counter target spell.
//! Set: DMR #50 — Dominaria Remastered | Scryfall ID: 89f612d6-7c59-4a7b-a87d-45f789e88ba5 | Oracle ID: 956381ba-6d37-4a8a-846c-bad79222dbee
// IMPLEMENTED — hard counter with pitch alternative (1 life + exile a blue
// card from hand) via the casting wizard.
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
    index: CardIndex::new(54),
    oracle_id: "956381ba-6d37-4a8a-846c-bad79222dbee",
    scryfall_id: "89f612d6-7c59-4a7b-a87d-45f789e88ba5",
    faces: &[FaceDef {
        name: "Force of Will",
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
                parts: &[CostPart::PayLife(1), CostPart::ExileFromHand(&BLUE_CARD)],
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::CounterTargetSpell],
        targets: Some(TargetReq::one(TargetSpec::Spell(&ANY_SPELL))),
    }],
};

#[cfg(test)]
mod tests {
    // Engine-level coverage in baylee-engine s7 tests: pitching (life +
    // exiled blue card) casts Force of Will with an empty mana pool.
}
