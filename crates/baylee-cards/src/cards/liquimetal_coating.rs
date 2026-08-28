//! Liquimetal Coating — {2} — Artifact
//! Oracle: {T}: Target permanent becomes an artifact in addition to its other types until end of turn.
//! Set: CM2 #197 — Commander Anthology Volume II | Scryfall ID: f631447c-36e3-4d82-a658-19c9767a216b | Oracle ID: f4bdc551-c2eb-4a34-a3e3-b4a017c925af
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(85),
    oracle_id: "f4bdc551-c2eb-4a34-a3e3-b4a017c925af",
    scryfall_id: "f631447c-36e3-4d82-a658-19c9767a216b",
    faces: &[FaceDef {
        name: "Liquimetal Coating",
        mana_cost: baylee_core::mana!("{2}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::EMPTY,
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
