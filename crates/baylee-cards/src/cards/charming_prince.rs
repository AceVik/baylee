//! Charming Prince — {1}{W} — Creature — Human Noble
//! Oracle: When this creature enters, choose one —
//! Oracle: • Scry 2.
//! Oracle: • You gain 3 life.
//! Oracle: • Exile another target creature you own. Return it to the battlefield under your control at the beginning of the next end step.
//! Set: FDN #568 — Foundations | Scryfall ID: aa7b47e1-7e32-4f2f-aecf-bac7ca197081 | Oracle ID: c48d844c-3976-4fa5-8e0d-3f0e535e7619
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(18),
    oracle_id: "c48d844c-3976-4fa5-8e0d-3f0e535e7619",
    scryfall_id: "aa7b47e1-7e32-4f2f-aecf-bac7ca197081",
    faces: &[FaceDef {
        name: "Charming Prince",
        mana_cost: baylee_core::mana!("{1}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::NOBLE],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
