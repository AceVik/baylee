//! Darksteel Forge — {9} — Artifact
//! Oracle: Artifacts you control have indestructible. (Effects that say "destroy" don't destroy them. Artifact creatures with indestructible can't be destroyed by damage.)
//! Set: 2XM #248 — Double Masters | Scryfall ID: 421089c4-c8d3-48c5-b313-fb1741546271 | Oracle ID: 9b3bec05-441f-4fdf-8b51-69fa8613fcd4
// IMPLEMENTED — indestructible grant to your artifacts (layer 6).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, Filter, KeywordSet, Layer, Modifier,
    PartnerKind, StaticAbility,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ARTIFACTS_YOURS: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasType(TypeSet::ARTIFACT)]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(31),
    oracle_id: "9b3bec05-441f-4fdf-8b51-69fa8613fcd4",
    scryfall_id: "421089c4-c8d3-48c5-b313-fb1741546271",
    faces: &[FaceDef {
        name: "Darksteel Forge",
        mana_cost: baylee_core::mana!("{9}"),
        types: TypeSet::ARTIFACT,
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Static(StaticAbility {
        layer: Layer::Ability,
        filter: ARTIFACTS_YOURS,
        modifier: Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE),
        cross_zone: false,
    })],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
