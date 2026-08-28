//! Darksteel Forge — {9} — Artifact
//! Oracle: Artifacts you control have indestructible. (Effects that say "destroy" don't destroy them. Artifact creatures with indestructible can't be destroyed by damage.)
//! Set: 2XM #248 — Double Masters | Scryfall ID: 421089c4-c8d3-48c5-b313-fb1741546271 | Oracle ID: 9b3bec05-441f-4fdf-8b51-69fa8613fcd4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(31),
    oracle_id: "9b3bec05-441f-4fdf-8b51-69fa8613fcd4",
    scryfall_id: "421089c4-c8d3-48c5-b313-fb1741546271",
    faces: &[FaceDef {
        name: "Darksteel Forge",
        mana_cost: baylee_core::mana!("{9}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
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
