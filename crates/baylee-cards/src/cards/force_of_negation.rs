//! Force of Negation — {1}{U}{U} — Instant
//! Oracle: If it's not your turn, you may exile a blue card from your hand rather than pay this spell's mana cost.
//! Oracle: Counter target noncreature spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
//! Set: 2X2 #50 — Double Masters 2022 | Scryfall ID: 1825a719-1b2a-4af9-9cd2-7cb497cd0317 | Oracle ID: ac2173f9-f223-440a-9231-fd98762bdc6f
// IMPLEMENTED — pitch on opponents' turns (wizard + NotYourTurn condition)
// and the counter-to-exile destination that separates it from Counterspell.
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
        alternative_costs: &[AlternativeCost {
            cost: Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::ExileFromHand(&BLUE_CARD)],
            },
            condition: AltCondition::NotYourTurn,
        }],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::CounterTargetSpellToExile],
        targets: Some(TargetReq::one(TargetSpec::Spell(&NONCREATURE_SPELL))),
    }],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {
    // Countering noncreature spells only; pitch only on opponents' turns.
}
