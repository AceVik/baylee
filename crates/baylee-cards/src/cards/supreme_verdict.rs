//! Supreme Verdict — {1}{W}{U}{U} — Sorcery
//! Oracle: This spell can't be countered.
//! Oracle: Destroy all creatures.
//! Set: RVR #67 — Ravnica Remastered | Scryfall ID: 3892f1c5-937e-4ef4-b6f9-e0c0ded070d0 | Oracle ID: 0230de18-8d15-4cfa-9d42-7ccddd9f9570
// IMPLEMENTED — uncounterable wrath.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURES: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(160),
    oracle_id: "0230de18-8d15-4cfa-9d42-7ccddd9f9570",
    scryfall_id: "3892f1c5-937e-4ef4-b6f9-e0c0ded070d0",
    faces: &[FaceDef {
        name: "Supreme Verdict",
        mana_cost: baylee_core::mana!("{1}{W}{U}{U}"),
        types: TypeSet::SORCERY,
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
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    keywords: KeywordSet::UNCOUNTERABLE,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::DestroyAll { filter: &CREATURES }],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {}
