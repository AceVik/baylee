//! Force of Negation — {1}{U}{U} — Instant
//! Oracle: If it's not your turn, you may exile a blue card from your hand rather than pay this spell's mana cost.
//! Oracle: Counter target noncreature spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
//! Set: 2X2 #50 — Double Masters 2022 | Scryfall ID: 1825a719-1b2a-4af9-9cd2-7cb497cd0317 | Oracle ID: ac2173f9-f223-440a-9231-fd98762bdc6f
// PARTIAL — pitch on opponents' turns implemented (wizard + NotYourTurn
// condition). NOT SUPPORTED yet: countered spell goes to exile instead of
// graveyard (needs per-source counter destination, M2.S7+).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, AltCondition, AlternativeCost, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static NONCREATURE_SPELL: Filter = Filter::LacksType(TypeSet::CREATURE);
static BLUE_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Blue]));

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(53),
    oracle_id: "ac2173f9-f223-440a-9231-fd98762bdc6f",
    scryfall_id: "1825a719-1b2a-4af9-9cd2-7cb497cd0317",
    faces: &[FaceDef {
        name: "Force of Negation",
        mana_cost: baylee_core::mana!("{1}{U}{U}"),
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
            condition: AltCondition::NotYourTurn,
        }],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("countered spell should go to exile (counter destination, M2.S7+)"),
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::CounterTargetSpell],
        targets: Some(TargetReq::one(TargetSpec::Spell(&NONCREATURE_SPELL))),
    }],
};

#[cfg(test)]
mod tests {
    // Countering noncreature spells only; pitch only on opponents' turns.
}
