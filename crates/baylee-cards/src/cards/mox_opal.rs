//! Mox Opal — {0} — Legendary Artifact
//! Oracle: Metalcraft — {T}: Add one mana of any color. Activate only if you control three or more artifacts.
//! Set: 2XM #275 — Double Masters | Scryfall ID: 56001a36-126b-4c08-af98-a6cc4d84210e | Oracle ID: de2440de-e948-4811-903c-0bbe376ff64d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(98),
    oracle_id: "de2440de-e948-4811-903c-0bbe376ff64d",
    scryfall_id: "56001a36-126b-4c08-af98-a6cc4d84210e",
    faces: &[FaceDef {
        name: "Mox Opal",
        mana_cost: baylee_core::mana!("{0}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::EMPTY,
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
