//! Vindicate — {1}{W}{B} — Sorcery
//! Oracle: Destroy target permanent.
//! Set: MH2 #294 — Modern Horizons 2 | Scryfall ID: 683c4e13-525c-45c9-8832-bfe67965c34e | Oracle ID: 63c1ac21-e3d8-40c2-8c09-3f31c52992ef
// IMPLEMENTED — destroy any target permanent (can't be regenerated).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_PERMANENT: Filter = Filter::Any;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(184),
    oracle_id: "63c1ac21-e3d8-40c2-8c09-3f31c52992ef",
    scryfall_id: "683c4e13-525c-45c9-8832-bfe67965c34e",
    faces: &[FaceDef {
        name: "Vindicate",
        mana_cost: baylee_core::mana!("{1}{W}{B}"),
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::Destroy {
            target: TargetSpec::Object(&ANY_PERMANENT),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&ANY_PERMANENT))),
    }],
};

#[cfg(test)]
mod tests {
    // Engine-level coverage via s4 scenario tests: the chosen permanent is
    // destroyed (battlefield → graveyard).
}
